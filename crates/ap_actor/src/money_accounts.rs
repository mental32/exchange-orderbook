use futures::StreamExt;
use matching_engine::asset_pair::DateTime;
use matching_engine::decimal::Decimal;
use matching_engine::money38_18::Money38_18;
use std::collections::HashMap;
use tracing::Instrument;

pub type UserDataId = i32;

pub type MoneyAccountId = i32;

#[derive(Debug)]
pub struct MoneyAccount {
    pub currency: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub user_id: Option<UserDataId>,
    pub fiat_source: Option<String>,
    pub crypto_source: Option<String>,
    pub balance: Money38_18,
}

pub async fn money_account_balances(
    pg_pool: sqlx::PgPool,
) -> HashMap<MoneyAccountId, MoneyAccount> {
    sqlx::query!(
        r#"
            SELECT
                ma.id as "account_id!",
                ma.currency as "currency!",
                ma.created_at as "created_at!",
                ma.updated_at as "updated_at!",
                ma.user_id as "user_id",
                ma.fiat_source as "fiat_source",
                ma.crypto_source as "crypto_source",
                COALESCE(SUM(CASE WHEN j.credit_account_id = ma.id THEN j.amount ELSE 0 END), 0) -
                COALESCE(SUM(CASE WHEN j.debit_account_id = ma.id THEN j.amount ELSE 0 END), 0) as "balance!: Decimal"
            FROM t_money_accounts ma
            LEFT JOIN t_account_tx_journal j ON
                j.credit_account_id = ma.id OR j.debit_account_id = ma.id
            GROUP BY ma.id
            "#
    )
    .fetch(&pg_pool)
    .filter_map(|result| async move {
        match result {
            Ok(row) => {
                let money_account = MoneyAccount {
                    currency: row.currency,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    user_id: row.user_id,
                    fiat_source: row.fiat_source,
                    crypto_source: row.crypto_source,
                    balance: Money38_18(row.balance),
                };

                Some((row.account_id, money_account))
            },
            Err(e) => {
                tracing::error!(?e, "failed to fetch account balance row");
                None
            }
        }
    })
    .collect()
    .instrument(tracing::error_span!("money_account_balances"))
    .await
}
