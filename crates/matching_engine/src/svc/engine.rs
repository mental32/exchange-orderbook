use super::ap_actor;
use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::BaseQuote;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderIndex;
use crate::orderbook::OrderSide;
use crate::place_order::PlaceOrderError;
use crate::reserve_money::ReserveMoney;
use crate::reserve_money::reserve_by_asset;
use crate::svc::routes;
use common_core::money38_18::Money38_18;
use common_core::web::middleware::clerk::ClerkUserId;
use futures::FutureExt as _;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::RecvError;

#[derive(Debug, thiserror::Error)]
#[error("order not found for user {0:?} and order uuid {1:?}")]
pub struct OrderNotFound(ClerkUserId, OrderUuid);

#[derive(Debug)]
pub struct SvcState {
    /// list of (base/quote, sender) tuples for each asset pair actor
    pub asset_processors: Vec<(
        BaseQuote,
        String, /* BaseQuote rendered */
        mpsc::Sender<ap_actor::Envelope>,
    )>,
    /// associate user ids with websocket sessions
    pub websocket_sessions: RwLock<Vec<(ClerkUserId, mpsc::Sender<routes::trade_ws::WsRpcMsg>)>>,
    /// the symbol vocabulary maps strings to [`AssetCode`]s (numbers)
    pub symbol_vocabulary: SymbolVocabulary,
}

#[derive(Debug, Clone)]
pub struct EngineFacade {
    pub pg_pool: sqlx::Pool<sqlx::Postgres>,
    pub inner: Arc<SvcState>,
}

impl EngineFacade {
    fn get_asset_processor_channel(
        &self,
        (base, quote): BaseQuote,
    ) -> Option<mpsc::Sender<ap_actor::Envelope>> {
        self.inner
            .asset_processors
            .iter()
            .find(|(base_quote, _, _)| *base_quote == (base, quote))
            .map(|(_, _, snd)| snd.clone())
    }
}

impl EngineFacade {
    pub async fn cancel_order(
        &self,
        order_uuid: OrderUuid,
        user_id: ClerkUserId,
    ) -> Result<oneshot::Receiver<ap_actor::Response>, OrderNotFound> {
        todo!();
        // let (order_index, base_quote) = match self.inner.order_uuids.get(&order_uuid).cloned() {
        //     Some((a, b)) => (a, b),
        //     None => {
        //         return Err(OrderNotFound(user_id, order_uuid));
        //     }
        // };

        // let Some((_, tx)) = self
        //     .inner
        //     .asset_processors
        //     .iter()
        //     .find(|(ap, _)| *ap == base_quote)
        //     .cloned()
        // else {
        //     tracing::error!(
        //         ?base_quote,
        //         "attempted to cancel order on non-existent asset pair"
        //     );
        //     return Err(OrderNotFound(user_id, order_uuid));
        // };

        // let co = crate::cancel_order::CancelOrder {
        //     user_id,
        //     order_uuid,
        //     order_index,
        // };

        // let (snd, rcv) = tokio::sync::oneshot::channel();
        // let envelope = (snd, vec![ap_actor::MessageIn::CancelOrder(co)]);
        // if let Err(err) = tx.send(envelope).await {
        //     tracing::error!(
        //         ?err,
        //         "error sending cancel order message to asset pair actor"
        //     );
        //     panic!("error sending cancel order message to asset pair actor");
        // }

        // match resp_rx.await {
        //     Ok(Ok(msg)) => {}
        //     Ok(Err(_)) => {
        //         return Err(OrderNotFound(user_uuid, order_uuid));
        //     }
        //     Err(err) => {
        //         tracing::error!(
        //             ?err,
        //             "error receiving cancel order response from asset pair actor"
        //         );
        //         panic!("error receiving cancel order response from asset pair actor");
        //     }
        // }

        Ok(todo!())
    }

    pub async fn place_order(
        &self,
        (base, quote): (AssetCode, AssetCode),
        user_id: ClerkUserId,
        order: routes::trade_add_order::TradeAddOrder,
    ) -> Result<OrderUuid, PlaceOrderError> {
        let Some(ap_snd) = self.get_asset_processor_channel((base, quote)) else {
            tracing::error!(
                base = ?base.as_str(&self.inner.symbol_vocabulary),
                quote = ?quote.as_str(&self.inner.symbol_vocabulary),
                "attempted to place order on non-existent asset pair"
            );
            return Err(PlaceOrderError::InvalidAssetPair);
        };

        let order_uuid = OrderUuid(uuid::Uuid::new_v4());

        let (msg_snd, msg_rcv) = oneshot::channel();
        let msg_in = ap_actor::MsgIn::PlaceOrder(crate::place_order::PlaceOrderArgs {
            base_quote: (base, quote),
            user_id,
            order_uuid,
            details: order,
        });

        if let Err(err) = ap_snd.send((msg_snd, msg_in)).await {
            tracing::error!(
                ?err,
                "error sending place order message to asset pair actor"
            );
            panic!("error sending place order message to asset pair actor");
        }

        match msg_rcv.await.expect("asset processor died") {
            Ok(ap_actor::MsgOut::PlacedOrder {}) => {
                tracing::info!(?order_uuid, "order placed");
                Ok(order_uuid.clone())
            }
            Ok(ap_actor::MsgOut::ValidatedOrder) => todo!(),

            Err(ap_actor::Error::UnserializableInput {
                base_quote,
                message,
                error,
            }) => todo!(),

            Ok(ap_actor::MsgOut::CancelOrder(_))
            | Ok(ap_actor::MsgOut::WillSuspend)
            | Ok(ap_actor::MsgOut::WillResume)
            | Ok(ap_actor::MsgOut::WillShutdown) => {
                unreachable!("bug: these outputs should never be received")
            }

            Err(ap_actor::Error::DbFailedToInsertMsgIn(error)) => todo!(),
            Err(ap_actor::Error::PlaceOrder(place_order_error)) => todo!(),
            Err(ap_actor::Error::NoReferencePrice) => todo!(),
            Err(ap_actor::Error::InsufficientFunds) => todo!(),
            Err(ap_actor::Error::ProcessorIsSuspended) => todo!(),
            Err(ap_actor::Error::InvalidPrice) => todo!(),
        }
    }

    pub fn is_pair_enabled(&self, symbol: &str) -> Option<BaseQuote> {
        self.inner
            .asset_processors
            .iter()
            .find(|(_, base_quote_st, _)| base_quote_st == symbol)
            .map(|(bq, _, _)| bq.clone())
    }
}
