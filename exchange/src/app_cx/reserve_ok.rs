use std::num::NonZeroU64;

use futures::TryFutureExt as _;

use super::{defer, DeferGuard};

#[derive(Debug, Clone)]
pub struct ReserveOk {
    pub row_id: u32,
    pub previous_balance: NonZeroU64,
    pub new_balance: Option<NonZeroU64>,
}

impl ReserveOk {
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

    pub fn revert(
        self,
        db: &sqlx::PgPool,
    ) -> impl std::future::Future<Output = Result<i32, sqlx::Error>> + '_ {
        sqlx::query!(
        r#"
            -- First, fetch the required details from the original row
            WITH original_tx AS (
            SELECT credit_account_id, debit_account_id, currency, amount
                FROM account_tx_journal
                WHERE id = $1
            )
            -- Then, insert the inverse transaction
            INSERT INTO account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type)
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
