use futures::StreamExt;
use tokio::sync::mpsc;

use crate::trading::{self, TradeCmd};
use crate::{Asset, Config};

pub struct SpawnTradingEngine {
    pub input: trading::TradingEngineTx,
    pub handle: tokio::task::JoinHandle<()>,
}

impl SpawnTradingEngine {
    pub async fn init_from_db(
        self,
        db: sqlx::PgPool,
    ) -> Result<(trading::TradingEngineTx, tokio::task::JoinHandle<()>), sqlx::Error> {
        let Self { input, handle } = self;

        // stream out rows from the orders_event_source table, deserialize them into TradeCmds
        // and send them to the trading engine for processing.
        let mut stream = sqlx::query!(r#"SELECT id, jstr FROM trading_event_source"#,).fetch(&db);

        while let Some(row) = stream.next().await {
            let row = row?;
            let cmd: trading::TradeCmdPayload = serde_json::from_value(row.jstr).unwrap();
            input
                .send(trading::TradingEngineCmd::Bootstrap(cmd))
                .await
                .unwrap();
        }

        Ok((input, handle))
    }
}

pub fn spawn_trading_engine(config: &Config, db: sqlx::PgPool) -> SpawnTradingEngine {
    use trading::TradingEngineCmd as T;

    async fn trading_engine_supervisor(mut rx: mpsc::Receiver<T>, db: sqlx::PgPool) {
        use trading::{AssetBook, Assets, TradeCmdPayload as P};

        let mut assets = Assets {
            order_uuids: Default::default(),
            eth: AssetBook::new(Asset::Ether),
            btc: AssetBook::new(Asset::Bitcoin),
        };

        macro_rules! try_event_log {
            ($input:expr, $e:expr) => {
                if let Ok(jstr) = ::serde_json::to_value(&$input) {
                    let res: Result<_, trading::TradingEngineError> = $e;

                    match sqlx::query!("INSERT INTO trading_event_source (jstr) VALUES ($1)", jstr)
                        .execute(&db)
                        .await
                    {
                        Ok(_) => res,
                        Err(e) => Err(trading::TradingEngineError::Database(e)),
                    }
                } else {
                    Err(trading::TradingEngineError::UnserializableInput)
                }
            };
        }
        let mut running = true;
        while let Some(cmd) = rx.recv().await {
            if !running {
                continue;
            }

            match cmd {
                T::Suspend => {
                    running = false;
                }
                T::Resume => {
                    running = true;
                }
                T::Shutdown => break,
                T::Trade(TradeCmd::PlaceOrder((place_order, response))) => {
                    let t = try_event_log!(
                        place_order,
                        trading::do_place_order(&mut assets, place_order)
                    );

                    let _ = response.send(t);
                }
                T::Trade(TradeCmd::CancelOrder((cancel_order, response))) => {
                    let t = try_event_log!(
                        cancel_order,
                        trading::do_cancel_order(&mut assets, cancel_order)
                    );

                    let _ = response.send(t);
                }
                T::Bootstrap(P::PlaceOrder(place_order)) => {
                    let _ = trading::do_place_order(&mut assets, place_order);
                }
                T::Bootstrap(P::CancelOrder(cancel_order)) => {
                    let _ = trading::do_cancel_order(&mut assets, cancel_order);
                }
            }
        }

        tracing::warn!("trading engine supervisor finished");
    }

    let (input, output) = mpsc::channel(config.te_channel_capacity());
    let handle = tokio::spawn(trading_engine_supervisor(output, db));

    SpawnTradingEngine { input, handle }
}
