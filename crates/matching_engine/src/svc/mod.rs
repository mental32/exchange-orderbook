//! service layer that exposes the matching engine as a http/rest api

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::asset_pair::BaseQuote;
use anyhow::Context as _;
use futures::StreamExt as _;
use futures::stream::FuturesUnordered;
use itertools::Itertools as _;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;

pub(crate) mod ap_actor;
pub(crate) mod engine;
pub(crate) mod routes;

fn launch_ap_procs(
    asset_pairs: &[AssetPairRow],
    pg_pool: sqlx::PgPool,
    symbol_vocabulary: SymbolVocabulary,
) -> Vec<(
    BaseQuote,
    tokio::sync::mpsc::Sender<ap_actor::Envelope>,
    tokio::task::JoinHandle<()>,
)> {
    asset_pairs
        .iter()
        .map(|ap_row| {
            let (snd, rcv) = tokio::sync::mpsc::channel(1);
            let base_quote = ap_row
                .base_quote(&symbol_vocabulary)
                .expect("base or quote was not in symbol vocabulary");
            let handle = tokio::task::spawn(
                ap_actor::asset_processor_loop(
                    rcv,
                    pg_pool.clone(),
                    ap_row.clone(),
                    symbol_vocabulary.clone(),
                )
                .instrument(tracing::info_span!(
                    "asset_processor_loop",
                    asset_pair = ap_row.id
                )),
            );
            (base_quote, snd, handle)
        })
        .collect()
}

/// Start the matching engine service layer, listening on the given address
///
pub async fn serve(
    pg_pool: sqlx::PgPool,
    bind_socket_addr: SocketAddr,
    signal: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let mut handles = vec![];

    // spawn an actor for each active asset pair
    let ap_rows = sqlx::query_as!(
        AssetPairRow,
        "SELECT * FROM t_trading_asset_pairs WHERE status = 'active'",
    )
    .fetch_all(&pg_pool)
    .await
    .context("error fetching active asset pairs")?;
    let symbol_vocabulary = ap_rows
        .iter()
        .map(|row| [row.base_asset.clone(), row.quote_asset.clone()])
        .flatten()
        .unique()
        .collect::<SymbolVocabulary>();

    let mut arc_state: Arc<engine::SvcState> = Arc::new(engine::SvcState {
        asset_processors: Default::default(),
        order_uuids: Default::default(),
        websocket_sessions: Default::default(),
        symbol_vocabulary,
    });

    let mut_ref_state = Arc::get_mut(&mut arc_state).expect("no other references to state");

    for (base_quote, tx, handle) in launch_ap_procs(
        &ap_rows,
        pg_pool.clone(),
        mut_ref_state.symbol_vocabulary.clone(),
    ) {
        mut_ref_state.asset_processors.push((base_quote, tx));
        handles.push(handle);
    }

    // stream out rows from the `t_trading_event_source` table, deserialize them into TradeCmds and process them
    {
        let mut stream =
            sqlx::query!(r#"SELECT id, jstr FROM t_trading_event_source"#,).fetch(&pg_pool);

        while let Some(maybe_row) = stream.next().await {
            match maybe_row {
                Ok(row) => {
                    tracing::info!(?row.id, "processing trading event source row");
                    let Ok((base_quote, msg_in)) = serde_json::from_value::<(
                        (AssetCode, AssetCode),
                        ap_actor::MessageIn,
                    )>(row.jstr) else {
                        tracing::error!(?row.id, "error deserializing trading event source row");
                        continue;
                    };

                    let Some((_, tx)) = mut_ref_state
                        .asset_processors
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

                    let (snd, rcv) = tokio::sync::oneshot::channel();
                    let envelope = (snd, vec![msg_in]);
                    if let Err(err) = tx.send(envelope).await {
                        tracing::error!(
                            ?err,
                            "error sending message from trading event source row to asset pair actor"
                        );
                        continue;
                    }

                    let msg_out = rcv
                        .await
                        .expect("recv-error from asset pair actor")
                        .expect("switch message failure");
                }
                Err(error) => {
                    tracing::error!(?error, "error fetching trading event source row");
                    return Err(error).context("error fetching trading event source row");
                }
            }
        }
    } // drop(stream)

    let ap_list = mut_ref_state.asset_processors.clone();

    let router = common_core::web::apply_middleware(routes::routes(engine::EngineFacade {
        pg_pool,
        inner: arc_state,
    }));

    let tcp_listener = tokio::net::TcpListener::bind(&bind_socket_addr).await?;
    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let app = axum::serve(tcp_listener, make_service).with_graceful_shutdown(signal);
    tracing::info!(?bind_socket_addr, "serving http router for trading");

    let res = app.await;

    // shutdown all ap actors concurrently
    {
        let futs = ap_list
            .iter()
            .map(|(_, tx)| {
                let (snd, rcv) = tokio::sync::oneshot::channel();
                let envelope = (snd, vec![ap_actor::MessageIn::Shutdown]);

                async move {
                    if let Err(err) = tx.send(envelope).await {
                        tracing::error!(?err, "error sending shutdown message to asset pair actor");
                    }

                    if let Err(err) = rcv.await {
                        tracing::error!(
                            ?err,
                            "error receiving shutdown response from asset pair actor"
                        );
                    };
                }
            })
            .collect::<FuturesUnordered<_>>();
        tokio::pin!(futs);

        while let Some(()) = futs.next().await {}

        futures::future::join_all(handles).await;
    }

    Ok(res?)
}
