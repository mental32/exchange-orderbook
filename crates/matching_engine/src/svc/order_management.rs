use super::ap_actor;
use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::BaseQuote;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderIndex;
use crate::orderbook::OrderSide;
use crate::pending_fill::TifViolation;
use crate::reserve_money::ReserveMoney;
use crate::reserve_money::reserve_money_by_asset;
use crate::svc::routes;
use axum::http::StatusCode;
use common_core::money38_18::Money38_18;
use common_core::web::middleware::clerk::ClerkUserId;
use futures::FutureExt as _;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use itertools::Itertools;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::RecvError;
use tokio::sync::watch;
use tonic::IntoRequest;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(in crate::svc) struct PlaceOrderArgs<D> {
    pub base_quote: BaseQuote,
    pub user_id: ClerkUserId,
    pub order_uuid: OrderUuid,
    pub order_details: D,
}

pub type PlaceOrderError = (StatusCode, &'static str);

impl From<ap_actor::Error> for PlaceOrderError {
    fn from(value: ap_actor::Error) -> Self {
        use ap_actor::Error as E;
        match value {
            E::UnserializableInput { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "Invalid input"),
            E::CouldNotPersistToEventSource(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not persist to event source",
            ),
            E::TifViolation(tif_violation) => match tif_violation {
                TifViolation::FillOrKill => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "order details caused Fill or Kill (FOK) violation",
                ),
                TifViolation::ImmediateOrCancel => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "order details caused Immediate or Cancel (IOC) violation",
                ),
            },
            E::ZeroQuantity => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Order quantity must be greater than zero",
            ),
            E::NoReferencePrice => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "No reference price available for relative pricing",
            ),
            E::InsufficientFunds => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Insufficient funds to place order",
            ),
            E::ProcessorIsSuspended => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Trading pair is currently suspended",
            ),
            E::InvalidPrice => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Computed order price is invalid (zero or negative)",
            ),
            E::OrderCancelled => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Order was cancelled during execution",
            ),
            E::NoOpenPositions => (StatusCode::NOT_FOUND, "No open positions for this user"),
            E::OrderNotFound => (StatusCode::NOT_FOUND, "Order not found"),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CancelOrderBy {
    TxId(OrderUuid),
    Userref(u32),
    ClientOrderId(String),
}

pub type CancelOrderError = (StatusCode, &'static str);

#[derive(Debug, thiserror::Error)]
#[error("order not found for user {0:?} and order uuid {1:?}")]
pub struct OrderNotFound(ClerkUserId, OrderUuid);

#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub base_quote: BaseQuote,
    pub order_uuid: OrderUuid,
    pub order_index: OrderIndex,
    pub userref: Option<u32>,
    pub cl_ord_id: Option<String>,
}

#[derive(Debug)]
pub struct UserProfile {
    pub open_orders: Vec<OpenOrder>,
}

impl UserProfile {
    pub fn isolate_orders_for_cancel(&self, cancel_order_by: &CancelOrderBy) -> Vec<OpenOrder> {
        match cancel_order_by {
            CancelOrderBy::TxId(order_uuid) => self
                .open_orders
                .iter()
                .find(|o| o.order_uuid == *order_uuid)
                .map(|o| vec![o.clone()])
                .unwrap_or_default(),
            CancelOrderBy::Userref(target) => self
                .open_orders
                .iter()
                .filter(|p| p.userref == Some(*target))
                .cloned()
                .collect(),
            CancelOrderBy::ClientOrderId(cl_ord_id) => self
                .open_orders
                .iter()
                .find(|p| {
                    p.cl_ord_id
                        .as_ref()
                        .map(|st| st == cl_ord_id)
                        .unwrap_or(false)
                })
                .map(|o| vec![o.clone()])
                .unwrap_or_default(),
        }
    }
}

pub type Tracking = watch::Sender<ahash::AHashMap<ClerkUserId, UserProfile>>;

#[derive(Debug)]
struct Inner {
    /// handles and senders to all running asset pair processors
    asset_processors: Vec<ap_actor::ApInfo>,
    /// associate user ids with websocket sessions
    websocket_sessions: RwLock<Vec<(ClerkUserId, mpsc::Sender<routes::trade_ws::WsRpcMsg>)>>,
    tracking: Tracking,
}

#[derive(Debug, Clone)]
pub(in crate::svc) struct OrderManagement {
    pub pg_pool: sqlx::PgPool,
    /// the symbol vocabulary maps strings to [`AssetCode`]s (numbers)
    pub symbol_vocabulary: SymbolVocabulary,
    pub inner: Arc<Inner>,
}

impl OrderManagement {
    pub(in crate::svc) fn new(
        pg_pool: sqlx::Pool<sqlx::Postgres>,
        symbol_vocabulary: SymbolVocabulary,
        asset_processors: Vec<ap_actor::ApInfo>,
    ) -> Self {
        Self {
            pg_pool,
            symbol_vocabulary,
            inner: Arc::new(Inner {
                asset_processors,
                tracking: watch::channel(Default::default()).0,
                websocket_sessions: RwLock::new(Vec::new()),
            }),
        }
    }

    fn get_asset_processor_channel(
        &self,
        (base, quote): BaseQuote,
    ) -> Option<mpsc::Sender<ap_actor::Envelope>> {
        self.inner
            .asset_processors
            .iter()
            .find(|i| i.base_quote == (base, quote))
            .map(|i| i.mpsc_sender.clone())
    }
}

impl OrderManagement {
    pub fn is_pair_enabled(&self, symbol: &str) -> Option<BaseQuote> {
        self.inner
            .asset_processors
            .iter()
            .find(|i| i.base_quote.0.as_str(&self.symbol_vocabulary) == symbol)
            .map(|i| i.base_quote.clone())
    }

    pub async fn cancel_order(
        &self,
        cancel_order_by: CancelOrderBy,
        clerk_user_id: ClerkUserId,
    ) -> Result<(Vec<uuid::Uuid>, Vec<(uuid::Uuid, String)>), CancelOrderError> {
        let mut pairs_to_contact = ahash::AHashMap::new();
        let cancelled = {};

        let mut success = vec![];
        let mut failed = vec![];

        let mut pending = FuturesUnordered::new();

        for open_order in self
            .inner
            .tracking
            .borrow()
            .get(&clerk_user_id)
            .map(|profile| profile.isolate_orders_for_cancel(&cancel_order_by))
            .unwrap_or_default()
            .into_iter()
        {
            pairs_to_contact
                .entry(open_order.base_quote)
                .or_insert(vec![])
                .push(open_order.order_uuid.clone());
        }

        for ((base, quote), ids) in pairs_to_contact {
            let Some(ap_snd) = self.get_asset_processor_channel((base, quote)) else {
                tracing::error!(
                    base = ?base.as_str(&self.symbol_vocabulary),
                    quote = ?quote.as_str(&self.symbol_vocabulary),
                    "attempted to cancel order on non-existent asset pair"
                );
                failed.extend(
                    ids.into_iter()
                        .map(|id| (id.0, "Asset pair not found".to_owned())),
                );
                continue;
            };

            let (msg_snd, msg_rcv) = oneshot::channel();
            let msg_in = ap_actor::MsgIn::CancelOrderBy {
                user_id: clerk_user_id.clone(),
                cancel_order_by: cancel_order_by.clone(),
            };

            pending.push(async move {
                let () = ap_snd
                    .send((msg_snd, msg_in))
                    .await
                    .map_err(|_| ids.clone())?;

                match msg_rcv.await.map_err(|_| ids.clone())? {
                    Ok(ap_actor::MsgOut::OrderCancelled { success, failed }) => {
                        Ok((success, failed))
                    }
                    Ok(t) => unreachable!("bug: these outputs should never be received"),
                    Err(_) => Err(ids),
                }
            });
        }

        tokio::pin!(pending);

        while let Some(res) = pending.next().await {
            let (s, f) = match res {
                Ok((s, f)) => (s, f),
                Err(f) => (vec![], f),
            };

            success.extend(s.into_iter().map(|id| id.0));

            failed.extend(
                f.into_iter()
                    .map(|id| (id.0, "Failed to cancel order".to_owned())), // TODO: the error message is generic i am aware. will fix later.
            )
        }

        Ok((success, failed))
    }

    pub async fn place_order(
        &self,
        (base, quote): (AssetCode, AssetCode),
        clerk_user_id: ClerkUserId,
        order_details: routes::trade_add_order::TradeAddOrder,
    ) -> Result<OrderUuid, PlaceOrderError> {
        let Some(ap_snd) = self.get_asset_processor_channel((base, quote)) else {
            tracing::error!(
                base = ?base.as_str(&self.symbol_vocabulary),
                quote = ?quote.as_str(&self.symbol_vocabulary),
                "attempted to place order on non-existent asset pair"
            );
            return Err((StatusCode::NOT_FOUND, "Asset pair not found"));
        };

        let order_uuid = OrderUuid(uuid::Uuid::new_v4());

        let (msg_snd, msg_rcv) = oneshot::channel();
        let msg_in = ap_actor::MsgIn::PlaceOrder(PlaceOrderArgs {
            base_quote: (base, quote),
            user_id: clerk_user_id.clone(),
            order_uuid,
            order_details,
        });

        if let Err(err) = ap_snd.send((msg_snd, msg_in)).await {
            tracing::error!(
                ?err,
                "error sending place order message to asset pair actor"
            );
            panic!("error sending place order message to asset pair actor");
        }

        match msg_rcv.await.map_err(|_recv_error| {
            tracing::error!("asset processor dropped without sending");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "asset processor dropped without sending",
            )
        })? {
            Ok(ap_actor::MsgOut::OrderPlaced) => {
                tracing::info!(?order_uuid, "order placed");

                Ok(order_uuid.clone())
            }
            Ok(ap_actor::MsgOut::OrderValidated) => todo!(),

            Ok(ap_actor::MsgOut::OrderCancelled { .. })
            | Ok(ap_actor::MsgOut::WillSuspend)
            | Ok(ap_actor::MsgOut::WillResume)
            | Ok(ap_actor::MsgOut::WillShutdown) => {
                unreachable!("bug: these outputs should never be received")
            }

            Err(error) => Err(PlaceOrderError::from(error)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_place_order_error_mappings() {
        // Test ZeroQuantity
        let error = ap_actor::Error::ZeroQuantity;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Order quantity must be greater than zero");

        // Test NoReferencePrice
        let error = ap_actor::Error::NoReferencePrice;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "No reference price available for relative pricing");

        // Test InsufficientFunds
        let error = ap_actor::Error::InsufficientFunds;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Insufficient funds to place order");

        // Test ProcessorIsSuspended (503 - the only non-422)
        let error = ap_actor::Error::ProcessorIsSuspended;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(message, "Trading pair is currently suspended");

        // Test InvalidPrice
        let error = ap_actor::Error::InvalidPrice;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            message,
            "Computed order price is invalid (zero or negative)"
        );

        // Test OrderCancelled
        let error = ap_actor::Error::OrderCancelled;
        let (status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Order was cancelled during execution");
    }
}
