//! Actor pattern for a single asset pair orderbook
//!
//! Each asset pair has its own actor that processes messages sequentially
//! and maintains its own state (the orderbook)

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::asset_pair::AssetPairRow;
use crate::orderbook::Orderbook;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Message {
    PlaceOrder(crate::place_order::PlaceOrder),
    CancelOrder(crate::cancel_order::CancelOrder),
    Suspend,
    Resume,
    Shutdown,
}

#[derive(Debug)]
pub enum MessageResult {
    PlaceOrder(crate::place_order::PlaceOrderResult),
    CancelOrder(Option<crate::orderbook::Order>),
}

pub type Envelope = (
    oneshot::Sender<Result<Option<MessageResult>, Error>>,
    Message,
);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("message serialization error")]
    UnserializableInput {
        base_quote: (AssetCode, AssetCode),
        message: Message,
        #[source]
        error: serde_json::Error,
    },
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("order placement error")]
    PlaceOrder(#[from] crate::place_order::PlaceOrderError),
    #[error("trading engine is suspended, cannot process message: {0:?}")]
    Suspended(Message),
}

struct Inner {
    pub rx: mpsc::Receiver<Envelope>,
    pub db: sqlx::PgPool,
    pub running: bool,
    pub orderbook: Orderbook,
    pub ap_row: AssetPairRow,
}

impl Inner {
    pub fn switch_msg(&mut self, msg: Message) -> Result<Option<MessageResult>, Error> {
        match msg {
            Message::PlaceOrder(po) => {
                let res = crate::place_order::do_place_order(&mut self.orderbook, po)?;
                Ok(Some(MessageResult::PlaceOrder(res)))
            }
            Message::CancelOrder(co) => {
                let maybe_order = crate::cancel_order::do_cancel_order(&mut self.orderbook, co);
                Ok(Some(MessageResult::CancelOrder(maybe_order)))
            }
            Message::Suspend => {
                self.running = false;
                Ok(None)
            }
            Message::Resume => {
                self.running = true;
                Ok(None)
            }
            Message::Shutdown => {
                // handle shutdown
                Ok(None)
            }
        }
    }
}

pub async fn task(rx: mpsc::Receiver<Envelope>, db: sqlx::PgPool, ap_row: AssetPairRow) {
    let mut proc = Inner {
        rx,
        db,
        running: true,
        orderbook: Orderbook::new_empty(),
        ap_row,
    };

    loop {
        match proc.rx.recv().await {
            Some((tx, msg)) => {
                if !proc.running {
                    tracing::warn!(?msg, "dropping message because processor is suspended");
                    let _ = tx.send(Err(Error::Suspended(msg)));
                    continue;
                } else {
                    let res = match serde_json::to_value((proc.ap_row.base_quote(), &msg)) {
                        Ok(jstr) => {
                            match sqlx::query!(
                                "INSERT INTO t_trading_event_source (jstr) VALUES ($1)",
                                jstr
                            )
                            .execute(&proc.db)
                            .await
                            {
                                Ok(_) => {
                                    let res: Result<_, Error> = proc.switch_msg(msg);
                                    res
                                }
                                Err(e) => Err(Error::Database(e)),
                            }
                        }
                        Err(error) => Err(Error::UnserializableInput {
                            base_quote: proc.ap_row.base_quote(),
                            message: msg,
                            error,
                        }),
                    };

                    if let Err(e) = tx.send(res) {
                        tracing::error!(
                            ?e,
                            "processor could not respond to message sender; the sender was dropped"
                        );
                    }
                }
            }
            None => {
                tracing::warn!("asset pair actor message channel closed");
                break;
            }
        }
    }
}
