//! Financial transaction rollback mechanism for order placement

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::decimal::Decimal;
use common_core::money38_18::Money38_18;
use common_core::web::middleware::clerk::ClerkUserId;
use futures::TryFutureExt as _;

/// A guard that executes a closure when dropped, unless explicitly cancelled
#[must_use]
pub struct DeferGuard<F: FnMut()> {
    f: F,
    active: bool,
}

// I have concerns about relying on `Drop` since there is no guarantee that the drop() glue gets called.
impl<F: FnMut()> Drop for DeferGuard<F> {
    fn drop(&mut self) {
        if self.active {
            (self.f)();
        }
    }
}

impl<F: FnMut()> DeferGuard<F> {
    /// Cancel the deferred operation, preventing it from executing on drop
    pub fn cancel(mut self) {
        self.active = false;
        std::mem::forget(self);
    }
}

/// Create a defer guard that will execute the given closure on drop
#[must_use]
pub fn defer<F: FnMut()>(f: F) -> DeferGuard<F> {
    DeferGuard { f, active: true }
}

/// Represents a successful fund reservation that can be automatically reverted
#[derive(Debug, Clone)]
pub struct ReserveMoney {
    /// Database row ID of the reservation transaction
    pub row_id: u32,
    /// User's balance before the reservation
    pub previous_balance: Decimal,
    /// User's balance after the reservation (None if fully reserved)
    pub new_balance: Option<Decimal>,
}

impl ReserveMoney {
    /// automatically revert this reservation if dropped without being explicitly cancelled
    pub fn defer_revert(
        self,
        handle: tokio::runtime::Handle,
        pg_pool: sqlx::PgPool,
    ) -> DeferGuard<impl FnMut()> {
        defer(move || {
            let this = self.clone();
            let pg_pool = pg_pool.clone();

            handle.spawn(async move {
                let fut = this.revert(&pg_pool);

                if let Err(err) = fut.await {
                    tracing::warn!(?err, "failed to revert reserved funds");
                }
            });
        })
    }

    /// Manually revert this fund reservation by creating an inverse transaction
    pub fn revert(
        self,
        pg_pool: &sqlx::PgPool,
    ) -> impl std::future::Future<Output = Result<i32, sqlx::Error>> + '_ {
        sqlx::query!(
            r#"
            -- First, fetch the required details from the original row
            WITH original_tx AS (
                SELECT credit_account_id, debit_account_id, currency, amount
                FROM t_account_tx_journal
                WHERE id = $1
            )
            -- Then, insert the inverse transaction
            INSERT INTO t_account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type)
            SELECT debit_account_id, credit_account_id, currency, amount, 'revert reserve asset'
            FROM original_tx
            RETURNING id
            "#,
            self.row_id as i32
        )
        .fetch_one(pg_pool)
        .map_ok(|rec| rec.id)
    }
}

pub async fn calculate_balance_from_user_and_currency(
    pg_pool: sqlx::PgPool,
    user_id: ClerkUserId,
    currency: AssetCode,
    symbol_vocabulary: &SymbolVocabulary,
) -> Result<Option<Money38_18>, sqlx::Error> {
    let rec = sqlx::query!(
        r#"SELECT f_calculate_balance($1, $2) as f_calculate_balance"#,
        user_id.0,
        currency.as_str(symbol_vocabulary)
    )
    .fetch_one(&pg_pool)
    .await?;

    return Ok(rec.f_calculate_balance.map(Money38_18));
}

#[derive(Debug, thiserror::Error)]
pub enum ReserveByAssetError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

pub async fn reserve_by_asset(
    pg_pool: sqlx::PgPool,
    user_id: ClerkUserId,
    Money38_18(quantity): Money38_18,
    currency: AssetCode,
    symbol_vocabulary: &SymbolVocabulary,
) -> Result<ReserveMoney, ReserveByAssetError> {
    let balance = calculate_balance_from_user_and_currency(
        pg_pool.clone(),
        user_id.clone(),
        currency,
        symbol_vocabulary,
    )
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
        quantity,
        user_id.0,
        currency.as_str(symbol_vocabulary),
    )
    .fetch_one(&pg_pool)
    .await?;
    let id: i32 = rec.id;

    tracing::trace!(id = id, ?user_id, "reserved USD fiat from user account");

    let new_balance = calculate_balance_from_user_and_currency(
        pg_pool,
        user_id.clone(),
        currency,
        symbol_vocabulary,
    )
    .await?;
    if let Some(nb) = &new_balance {
        // Money38_18 implements Ord via the inner Decimal, compare by reference
        assert!(nb < &balance);
    }

    Ok(ReserveMoney {
        row_id: id as u32,
        previous_balance: balance.0,
        new_balance: new_balance.map(|m| m.0),
    })
}
