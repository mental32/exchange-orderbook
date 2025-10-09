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
use tokio::sync::oneshot;
use tracing::Instrument;

pub(crate) mod ap_actor;
pub(crate) mod engine;
pub(crate) mod routes;

fn launch_ap_procs(
    asset_pairs: &[AssetPairRow],
    pg_pool: sqlx::PgPool,
    symbol_vocabulary: SymbolVocabulary,
    channel_buffer_size: usize,
) -> Vec<(
    BaseQuote,
    String, /* BaseQuote rendered */
    tokio::sync::mpsc::Sender<ap_actor::Envelope>,
    tokio::task::JoinHandle<()>,
)> {
    asset_pairs
        .iter()
        .map(|ap_row| {
            let (snd, rcv) = tokio::sync::mpsc::channel(channel_buffer_size);
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
            let base_quote_st = format!(
                "{base}/{quote}",
                base = ap_row.base_asset,
                quote = ap_row.quote_asset
            );
            (base_quote, base_quote_st, snd, handle)
        })
        .collect()
}

/// Start the matching engine service layer, listening on the given address
///
pub async fn serve(
    pg_pool: sqlx::PgPool,
    bind_socket_addr: SocketAddr,
    graceful_shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let mut handles = vec![];

    let tcp_listener = tokio::net::TcpListener::bind(&bind_socket_addr).await?;

    // spawn an actor for each active asset pair
    let asset_pairs = sqlx::query_as!(
        AssetPairRow,
        "SELECT * FROM t_trading_asset_pairs WHERE status = 'active'",
    )
    .fetch_all(&pg_pool)
    .await
    .context("error fetching active asset pairs")?;
    let symbol_vocabulary = asset_pairs
        .iter()
        .map(|r| [r.base_asset.clone(), r.quote_asset.clone()])
        .flatten()
        .unique()
        .collect::<SymbolVocabulary>();

    let mut arc_state: Arc<engine::SvcState> = Arc::new(engine::SvcState {
        asset_processors: Default::default(),
        websocket_sessions: Default::default(),
        symbol_vocabulary,
    });

    let mut_ref_state = Arc::get_mut(&mut arc_state).expect("no other references to state");

    let channel_buffer_size = option_env!("AP_CHANNEL_BUFFER_SIZE")
        .and_then(|st| {
            st.parse()
                .inspect_err(|err| {
                    tracing::error!(?err, "The environment variable AP_CHANNEL_BUFFER_SIZE could not be parsed to a number (usize)");
                })
                .ok()
        })
        .unwrap_or(1);

    for (base_quote, base_quote_st, tx, handle) in launch_ap_procs(
        &asset_pairs,
        pg_pool.clone(),
        mut_ref_state.symbol_vocabulary.clone(),
        channel_buffer_size,
    ) {
        mut_ref_state
            .asset_processors
            .push((base_quote, base_quote_st, tx));
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
                        ap_actor::MsgIn,
                    )>(row.jstr) else {
                        tracing::error!(?row.id, "error deserializing trading event source row");
                        continue;
                    };

                    let Some((_, _, snd)) = mut_ref_state
                        .asset_processors
                        .iter()
                        .find(|(ap, _, _)| *ap == base_quote)
                        .cloned()
                    else {
                        tracing::error!(
                            ?base_quote,
                            "no asset pair actor found for trading event source row"
                        );
                        continue;
                    };

                    let (msg_snd, msg_rcv) = oneshot::channel();
                    let envelope = (msg_snd, msg_in);
                    if let Err(err) = snd.send(envelope).await {
                        tracing::error!(
                            ?err,
                            "error sending message from trading event source row to asset pair actor"
                        );
                        continue;
                    }

                    let msg_out = msg_rcv
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

    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let app =
        axum::serve(tcp_listener, make_service).with_graceful_shutdown(graceful_shutdown_signal);
    tracing::info!(?bind_socket_addr, "serving http router for trading");

    let res = app.await;

    // shutdown all ap actors concurrently
    {
        let futs = ap_list
            .iter()
            .map(|(_, _, snd)| {
                let (msg_snd, msg_rcv) = oneshot::channel();
                let envelope = (msg_snd, ap_actor::MsgIn::Shutdown);

                async move {
                    if let Err(err) = snd.send(envelope).await {
                        tracing::error!(?err, "error sending shutdown message to asset pair actor");
                    }

                    if let Err(err) = msg_rcv.await {
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
