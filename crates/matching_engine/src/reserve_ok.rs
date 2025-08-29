//! Financial transaction rollback mechanism for order placement

use crate::decimal::Decimal;
use futures::TryFutureExt as _;

/// A guard that executes a closure when dropped, unless explicitly cancelled
#[must_use]
pub struct DeferGuard<F: FnMut()> {
    f: F,
    active: bool,
}

// I am vert worried about relying on Drop since drop is not actually guaranteed to be called.
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
    }
}

/// Create a defer guard that will execute the given closure on drop
#[must_use]
pub fn defer<F: FnMut()>(f: F) -> DeferGuard<F> {
    DeferGuard { f, active: true }
}

/// Represents a successful fund reservation that can be automatically reverted
#[derive(Debug, Clone)]
pub struct ReserveOk {
    /// Database row ID of the reservation transaction
    pub row_id: u32,
    /// User's balance before the reservation
    pub previous_balance: Decimal,
    /// User's balance after the reservation (None if fully reserved)
    pub new_balance: Option<Decimal>,
}

impl ReserveOk {
    /// Create a defer guard that will automatically revert this reservation
    /// if dropped without being explicitly cancelled
    pub fn defer_revert(
        self,
        handle: tokio::runtime::Handle,
        db: sqlx::PgPool,
    ) -> DeferGuard<impl FnMut()> {
        defer(move || {
            let this = self.clone();
            let db = db.clone();

            handle.spawn(async move {
                let fut = this.revert(&db);

                if let Err(err) = fut.await {
                    tracing::warn!(?err, "failed to revert reserved funds");
                }
            });
        })
    }

    /// Manually revert this fund reservation by creating an inverse transaction
    pub fn revert(
        self,
        db: &sqlx::PgPool,
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
        .fetch_one(db)
        .map_ok(|rec| rec.id)
    }
}
