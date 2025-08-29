use super::ap_actor;
use crate::asset_code::AssetCode;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderIndex;
use crate::orderbook::OrderSide;
use crate::place_order::PlaceOrderError;
use crate::reserve_ok::ReserveOk;
use crate::svc::routes;
use common_core::money38_18::Money38_18;
use common_core::web::middleware::clerk::ClerkUserId;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error)]
pub enum ReserveByAssetError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("order not found for user {0:?} and order uuid {1:?}")]
pub struct OrderNotFound(ClerkUserId, OrderUuid);

#[derive(Debug, Clone)]
pub struct MatchingEngineFacade {
    /// the database connection pool
    pub pool: sqlx::Pool<sqlx::Postgres>,
    /// list of (base/quote, sender) tuples for each asset pair actor
    pub ap_list: Vec<((AssetCode, AssetCode), mpsc::Sender<ap_actor::Envelope>)>,
    /// map of order uuids to order indexes and assets.
    pub order_uuids: ahash::AHashMap<OrderUuid, (OrderIndex, (AssetCode, AssetCode))>,
}

impl MatchingEngineFacade {
    pub async fn calculate_balance_from_accounting(
        &self,
        user_id: ClerkUserId,
        currency: AssetCode,
    ) -> Result<Option<Money38_18>, sqlx::Error> {
        let rec = sqlx::query!(
            r#"SELECT f_calculate_balance($1, $2) as f_calculate_balance"#,
            user_id.0,
            currency.to_string()
        )
        .fetch_one(&self.pool)
        .await?;

        return Ok(rec.f_calculate_balance.map(Money38_18));
    }

    pub async fn reserve_by_asset(
        &self,
        user_id: ClerkUserId,
        quantity: Money38_18,
        currency: AssetCode,
    ) -> Result<crate::reserve_ok::ReserveOk, ReserveByAssetError> {
        let balance = self
            .calculate_balance_from_accounting(user_id.clone(), currency)
            .await?
            .expect("i dont know in what scenario this would be none");

        // create a new account_tx_journal record to debit the user's account for the reserved amount.
        let rec = sqlx::query!(
            r#"
            INSERT INTO t_account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type) VALUES (
                (SELECT id FROM t_money_accounts WHERE source_type = 'fiat' AND source_id = 'exchange' AND currency = $3),
                (SELECT id FROM t_money_accounts WHERE source_type = 'user' AND source_id = $2),
        $3,
        $1::numeric,
                'reserve asset'
            ) RETURNING id
            "#,
            quantity.0,
            user_id.0,
            currency.to_string(),
        )
        .fetch_one(&self.pool)
        .await?;
        let id: i32 = rec.id;

        tracing::trace!(id = id, ?user_id, "reserved USD fiat from user account");

        let new_balance = self
            .calculate_balance_from_accounting(user_id, currency)
            .await?;
        if let Some(nb) = &new_balance {
            // Money38_18 implements Ord via the inner Decimal, compare by reference
            assert!(nb < &balance);
        }

        Ok(crate::reserve_ok::ReserveOk {
            row_id: id as u32,
            previous_balance: balance.0,
            new_balance: new_balance.map(|m| m.0),
        })
    }

    pub async fn cancel_order(
        &self,
        order_uuid: OrderUuid,
        user_id: ClerkUserId,
    ) -> Result<
        oneshot::Receiver<Result<Option<ap_actor::MessageResult>, ap_actor::Error>>,
        OrderNotFound,
    > {
        let (order_index, base_quote) = match self.order_uuids.get(&order_uuid).cloned() {
            Some((a, b)) => (a, b),
            None => {
                return Err(OrderNotFound(user_id, order_uuid));
            }
        };

        let Some((_, tx)) = self
            .ap_list
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

        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        let envelope = (resp_tx, ap_actor::Message::CancelOrder(co));
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

        Ok(resp_rx)
    }

    pub async fn place_order(
        &self,
        base_quote: (AssetCode, AssetCode),
        user_id: ClerkUserId,
        order: routes::trade_add_order::TradeAddOrder,
    ) -> Result<
        (
            oneshot::Receiver<Result<Option<ap_actor::MessageResult>, ap_actor::Error>>,
            ReserveOk,
        ),
        PlaceOrderError,
    > {
        let Some((_, tx)) = self
            .ap_list
            .iter()
            .find(|(ap, _)| *ap == base_quote)
            .cloned()
        else {
            tracing::error!(
                ?base_quote,
                "attempted to place order on non-existent asset pair"
            );
            return Err(PlaceOrderError::InvalidAssetPair);
        };

        let (base, quote) = base_quote;
        let quantity = order.quantity;

        // convert Decimal quantity into Money38_18 for reservations
        let qty_money = Money38_18(quantity);

        let reserve = match order.side {
            OrderSide::Buy => {
                self.reserve_by_asset(user_id.clone(), qty_money.clone(), quote)
                    .await?
            }
            OrderSide::Sell => {
                self.reserve_by_asset(user_id.clone(), qty_money.clone(), base)
                    .await?
            }
        };

        tracing::trace!(?reserve.previous_balance, ?reserve.new_balance, "marked funds as reserved");

        let po = crate::place_order::PlaceOrder {
            base_quote,
            user_id,
            side: order.side,
            order_type: order.order_type,
            quantity: order.quantity,
            price: order.price,
            time_in_force: order.time_in_force,
            stp: order.stp,
        };

        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        let envelope = (resp_tx, ap_actor::Message::PlaceOrder(po));
        if let Err(err) = tx.send(envelope).await {
            tracing::error!(
                ?err,
                "error sending place order message to asset pair actor"
            );
            panic!("error sending place order message to asset pair actor");
        }

        return Ok((resp_rx, reserve));
    }

    pub fn db(&self) -> sqlx::Pool<sqlx::Postgres> {
        self.pool.clone()
    }

    pub fn is_pair_enabled(&self, base_quote: String) -> Option<(AssetCode, AssetCode)> {
        self.ap_list
            .iter()
            .find(|((base, quote), _)| format!("{base}/{quote}") == base_quote)
            .map(|(ap, _)| ap.clone())
    }
}
