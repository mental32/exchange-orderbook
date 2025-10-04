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

// ... I am still deciding what type of map I want to use
type Map<K, V> = ahash::AHashMap<K, V>;

#[derive(Debug, thiserror::Error)]
#[error("order not found for user {0:?} and order uuid {1:?}")]
pub struct OrderNotFound(ClerkUserId, OrderUuid);

#[derive(Debug)]
pub struct SvcState {
    /// list of (base/quote, sender) tuples for each asset pair actor
    pub asset_processors: Vec<((AssetCode, AssetCode), mpsc::Sender<ap_actor::Envelope>)>,
    /// associate order uuids with order indexes and asset codes
    pub order_uuids: Map<OrderUuid, (OrderIndex, (AssetCode, AssetCode))>,
    /// associate user ids with websocket sessions
    pub websocket_sessions: RwLock<Vec<(ClerkUserId, mpsc::Sender<routes::trade_ws::SystemMsg>)>>,
    /// symbol vocabulary maps strings to numbers that [`AssetCode`] uses.
    pub symbol_vocabulary: SymbolVocabulary,
}

#[derive(Debug, Clone)]
pub struct EngineFacade {
    pub pg_pool: sqlx::Pool<sqlx::Postgres>,
    pub inner: Arc<SvcState>,
}

impl EngineFacade {
    pub async fn cancel_order(
        &self,
        order_uuid: OrderUuid,
        user_id: ClerkUserId,
    ) -> Result<oneshot::Receiver<ap_actor::Response>, OrderNotFound> {
        let (order_index, base_quote) = match self.inner.order_uuids.get(&order_uuid).cloned() {
            Some((a, b)) => (a, b),
            None => {
                return Err(OrderNotFound(user_id, order_uuid));
            }
        };

        let Some((_, tx)) = self
            .inner
            .asset_processors
            .iter()
            .find(|(ap, _)| *ap == base_quote)
            .cloned()
        else {
            tracing::error!(
                ?base_quote,
                "attempted to cancel order on non-existent asset pair"
            );
            return Err(OrderNotFound(user_id, order_uuid));
        };

        let co = crate::cancel_order::CancelOrder {
            user_id,
            order_uuid,
            order_index,
        };

        let (snd, rcv) = tokio::sync::oneshot::channel();
        let envelope = (snd, vec![ap_actor::MessageIn::CancelOrder(co)]);
        if let Err(err) = tx.send(envelope).await {
            tracing::error!(
                ?err,
                "error sending cancel order message to asset pair actor"
            );
            panic!("error sending cancel order message to asset pair actor");
        }

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

        Ok(rcv)
    }

    pub async fn place_order(
        &self,
        (base, quote): (AssetCode, AssetCode),
        user_id: ClerkUserId,
        order: routes::trade_add_order::TradeAddOrder,
    ) -> Result<OrderUuid, PlaceOrderError> {
        let Some((_, ap_snd)) = self
            .inner
            .asset_processors
            .iter()
            .find(|(ap, _)| *ap == (base, quote))
            .cloned()
        else {
            tracing::error!(
                base = ?base.as_str(&self.inner.symbol_vocabulary),
                quote = ?quote.as_str(&self.inner.symbol_vocabulary),
                "attempted to place order on non-existent asset pair"
            );
            return Err(PlaceOrderError::InvalidAssetPair);
        };

        let order_uuid = crate::order_uuid::OrderUuid(uuid::Uuid::new_v4());

        let order_index = {
            let (msg_snd, msg_rcv) = tokio::sync::oneshot::channel();
            let envelope = (
                msg_snd,
                vec![ap_actor::MessageIn::PlaceOrder(
                    crate::place_order::PlaceOrderArgs {
                        base_quote: (base, quote),
                        user_id,
                        order_uuid,
                        details: order,
                    },
                )],
            );
            if let Err(err) = ap_snd.send(envelope).await {
                tracing::error!(
                    ?err,
                    "error sending place order message to asset pair actor"
                );
                panic!("error sending place order message to asset pair actor");
            }

            let ap_actor::MessageOut::PlaceOrder(crate::place_order::PlaceOrderResult {
                order_index,
                original_args: request,
                ..
            }) = msg_rcv
                .map(
                    |result: Result<ap_actor::Response, RecvError>| match result {
                        Ok(Ok(msg_out)) => msg_out[0].clone(),
                        Ok(Err(err)) => todo!("asset processor error: {}", err),
                        Err(RecvError) => panic!("asset processor may have died"),
                    },
                )
                .await
            else {
                panic!("unexpected message result from asset pair actor");
            };

            order_index
        };

        // If the order has a resting portion in the orderbook, store the UUID mapping for cancellation
        if let Some(order_index) = order_index {
            // engine
            //     .inner
            //     .order_uuids
            //     .insert(*order_uuid, (*order_index, base_quote));
            tracing::debug!(
                ?order_uuid,
                ?order_index,
                "stored UUID mapping for order tracking"
            );
        }

        tracing::info!(?order_uuid, "order placed");

        Ok(order_uuid.clone())
    }

    pub fn is_pair_enabled(&self, base_quote: &str) -> Option<BaseQuote> {
        self.inner
            .asset_processors
            .iter()
            .find(|((base, quote), _)| {
                format!(
                    "{base}/{quote}",
                    base = base.as_str(&self.inner.symbol_vocabulary),
                    quote = quote.as_str(&self.inner.symbol_vocabulary)
                ) == base_quote
            })
            .map(|(ap, _)| ap.clone())
    }
}
