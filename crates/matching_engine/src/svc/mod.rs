//! service layer that exposes the matching engine as a http/rest api

use crate::asset_code::AssetCode;
use crate::asset_pair::AssetPairRow;
use crate::asset_pair::BaseQuote;
use anyhow::Context as _;
use futures::StreamExt as _;
use std::net::SocketAddr;

pub(crate) mod ap_actor;
pub(crate) mod engine;
pub(crate) mod routes;

fn launch_ap_procs(
    asset_pairs: &[AssetPairRow],
    db_pool: sqlx::PgPool,
) -> Vec<(
    BaseQuote,
    tokio::sync::mpsc::Sender<ap_actor::Envelope>,
    tokio::task::JoinHandle<()>,
)> {
    asset_pairs
        .iter()
        .map(|ap_row| {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let base_quote = ap_row.base_quote();
            let handle = tokio::task::spawn(ap_actor::task(rx, db_pool.clone(), ap_row.clone()));
            (base_quote, tx, handle)
        })
        .collect()
}

/// Start the matching engine service layer, listening on the given address
///
pub async fn serve(executor: sqlx::PgPool, bind_socket_addr: SocketAddr) -> anyhow::Result<()> {
    let mut engine = engine::MatchingEngineFacade {
        pool: executor,
        ap_list: vec![],
        order_uuids: Default::default(),
    };

    let mut handles = vec![];

    // spawn an actor for each active asset pair
    let ap_rows = sqlx::query_as!(
        AssetPairRow,
        "SELECT * FROM t_trading_asset_pairs WHERE status = 'active'",
    )
    .fetch_all(&engine.pool)
    .await
    .context("error fetching active asset pairs")?;
    for (base_quote, tx, handle) in launch_ap_procs(&ap_rows, engine.pool.clone()) {
        engine.ap_list.push((base_quote, tx));
        handles.push(handle);
    }

    // stream out rows from the `orders_event_source` table, deserialize them into TradeCmds and process them
    {
        let mut stream =
            sqlx::query!(r#"SELECT id, jstr FROM t_trading_event_source"#,).fetch(&engine.pool);

        while let Some(maybe_row) = stream.next().await {
            match maybe_row {
                Ok(row) => {
                    tracing::info!(?row.id, "processing trading event source row");
                    let Ok((base_quote, msg)) = serde_json::from_value::<(
                        (AssetCode, AssetCode),
                        ap_actor::Message,
                    )>(row.jstr) else {
                        tracing::error!(?row.id, "error deserializing trading event source row");
                        continue;
                    };

                    let Some((_, tx)) = engine
                        .ap_list
                        .iter()
                        .find(|(ap, _)| *ap == base_quote)
                        .cloned()
                    else {
                        tracing::error!(
                            ?base_quote,
                            "no asset pair actor found for trading event source row"
                        );
                        continue;
                    };

                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let envelope = (resp_tx, msg);
                    if let Err(err) = tx.send(envelope).await {
                        tracing::error!(
                            ?err,
                            "error sending message from trading event source row to asset pair actor"
                        );
                        continue;
                    }

                    let Ok(Ok(_)) = resp_rx.await else {
                        tracing::error!(
                            "error receiving response from asset pair actor for trading event source row"
                        );
                        continue;
                    };
                }
                Err(error) => {
                    tracing::error!(?error, "error fetching trading event source row");
                    return Err(error).context("error fetching trading event source row");
                }
            }
        }
    } // drop(stream)

    let ap_list = engine.ap_list.clone();

    let router = common_core::web::apply_middleware(routes::routes(engine));

    let tcp_listener = tokio::net::TcpListener::bind(&bind_socket_addr).await?;
    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let app = axum::serve(tcp_listener, make_service);
    tracing::info!(?bind_socket_addr, "serving http router for trading");

    let res = app.await;

    // shutdown all ap actors
    for (_, tx) in ap_list {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        let envelope = (resp_tx, ap_actor::Message::Shutdown);
        if let Err(err) = tx.send(envelope).await {
            tracing::error!(?err, "error sending shutdown message to asset pair actor");
            continue;
        }

        let Ok(Ok(_)) = resp_rx.await else {
            tracing::error!("error receiving shutdown response from asset pair actor");
            continue;
        };
    }

    futures::future::join_all(handles).await;

    // TODO: wind down? Should we even bother waiting for ap's to shut down?

    Ok(res?)
}

#[cfg(test)]
mod tests;
