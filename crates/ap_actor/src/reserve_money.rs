//! Financial transaction rollback mechanism for order placement

use futures::TryFutureExt as _;
use matching_engine::asset_code::AssetCode;
use matching_engine::asset_code::SymbolVocabulary;
use matching_engine::decimal::Decimal;
use matching_engine::money38_18::Money38_18;

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
    /// Manually revert this fund reservation by creating an inverse transaction
    pub fn revert(
        self,
        pg_pool: &sqlx::PgPool,
    ) -> impl std::future::Future<Output = Result<i32, sqlx::Error>> + '_ {
        let txid = uuid::Uuid::new_v4().to_string();
        sqlx::query!(
            r#"
            -- First, fetch the required details from the original row
            WITH original_tx AS (
                SELECT credit_account_id, debit_account_id, currency, amount
                FROM t_account_tx_journal
                WHERE id = $1
            )
            -- Then, insert the inverse transaction
            INSERT INTO t_account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            SELECT debit_account_id, credit_account_id, currency, amount, 'revert reserve asset', $2
            FROM original_tx
            RETURNING id
            "#,
            self.row_id as i32,
            txid
        )
        .fetch_one(pg_pool)
        .map_ok(|rec| rec.id)
    }
}

pub async fn calculate_balance_from_user_and_currency<'a>(
    executor: &'a mut sqlx::PgConnection,
    user_id: &str,
    currency: &AssetCode,
    _symbol_vocabulary: &'a SymbolVocabulary,
) -> Result<Option<Money38_18>, sqlx::Error> {
    let user_id_int: i32 = user_id.parse().map_err(|_| sqlx::Error::RowNotFound)?;
    let currency_str = currency.as_str();

    let rec = sqlx::query!(
        r#"
        SELECT COALESCE(
            (SELECT SUM(amount) FROM t_account_tx_journal
             WHERE credit_account_id = (SELECT id FROM t_money_accounts WHERE user_id = $1 AND currency = $2)),
            0
        ) - COALESCE(
            (SELECT SUM(amount) FROM t_account_tx_journal
             WHERE debit_account_id = (SELECT id FROM t_money_accounts WHERE user_id = $1 AND currency = $2)),
            0
        ) as "balance!"
        "#,
        user_id_int,
        currency_str
    )
    .fetch_one(executor)
    .await?;

    Ok(Some(Money38_18(rec.balance)))
}

#[derive(Debug, thiserror::Error)]
pub enum ReserveByAssetError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("account not found")]
    AccountNotFound,
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

pub async fn reserve_money_by_asset(
    executor: &mut sqlx::PgConnection,
    user_id: &str,
    Money38_18(quantity): Money38_18,
    currency: &AssetCode,
    _symbol_vocabulary: &SymbolVocabulary,
) -> Result<ReserveMoney, ReserveByAssetError> {
    let user_id_int: i32 = user_id
        .parse()
        .map_err(|_| ReserveByAssetError::AccountNotFound)?;
    let currency_str = currency.as_str();

    // Determine if currency is fiat or crypto (simple heuristic: USD/EUR/GBP are fiat)
    let is_fiat = matches!(currency_str, "USD" | "EUR" | "GBP");

    let rec = sqlx::query!(
        r#"
        WITH
          user_account AS (
            SELECT id FROM t_money_accounts WHERE user_id = $1 AND currency = $2
          ),
          exchange_account AS (
            SELECT id FROM t_money_accounts
            WHERE currency = $2 AND (
                ($3 AND fiat_source = 'exchange') OR
                (NOT $3 AND crypto_source IS NOT NULL)
            )
            LIMIT 1
          ),
          initial_balance AS (
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = (SELECT id FROM user_account)),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = (SELECT id FROM user_account)),
                0
            ) as balance
          ),
          reservation AS (
            INSERT INTO t_account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            SELECT
                (SELECT id FROM exchange_account),
                (SELECT id FROM user_account),
                $2,
                $4::numeric,
                'reserve asset',
                $5
            FROM initial_balance
            RETURNING id, (SELECT balance FROM initial_balance) as prev_bal
          ),
          new_balance AS (
            SELECT prev_bal - $4::numeric as balance
            FROM reservation
          )
        SELECT
          (SELECT id FROM reservation) as "id!",
          (SELECT prev_bal FROM reservation) as "previous_balance!",
          (SELECT balance FROM new_balance) as "new_balance"
        "#,
        user_id_int,
        currency_str,
        is_fiat,
        quantity,
        uuid::Uuid::new_v4().to_string()
    )
    .fetch_one(&mut *executor)
    .await?;

    tracing::trace!(id = rec.id, ?user_id, "reserved funds from user account");

    Ok(ReserveMoney {
        row_id: rec.id as u32,
        previous_balance: rec.previous_balance,
        new_balance: rec.new_balance,
    })
}

#[cfg(test)]
mod test {
    use crate::reserve_money::calculate_balance_from_user_and_currency;
    use crate::reserve_money::reserve_money_by_asset;
    use matching_engine::asset_code::AssetCode;
    use matching_engine::asset_code::SymbolVocabulary;
    use matching_engine::decimal::Decimal;
    use matching_engine::money38_18::Money38_18;
    use uuid::Uuid;

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_reserve_money_by_asset(pg_pool: sqlx::PgPool) {
        // Setup: Create test user
        let user_tag = Uuid::new_v4();
        let clerk_id = format!("reserve-clerk-{user_tag}");
        let name = format!("Test Reserve User {user_tag}");
        let email = format!("test-reserve-{user_tag}@example.com");

        let user_data_id = sqlx::query_scalar!(
            "INSERT INTO t_user_data (clerk, tier, name, email, password_hash) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            clerk_id,
            1,
            name,
            email,
            &[] as &[u8]
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        let user_id = user_data_id.to_string();

        // Setup: Create symbol vocabulary and currency
        let symbol_vocabulary = SymbolVocabulary::from_iter(vec!["USD".into()]);
        let currency = AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap();

        // Setup: Create user USD account
        let usd_account_id = sqlx::query_scalar!(
            "INSERT INTO t_money_accounts (currency, user_id) VALUES ($1, $2) RETURNING id",
            "USD",
            user_data_id
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        // Setup: Get exchange USD account (from migration)
        let exchange_usd_account_id = sqlx::query_scalar!(
            "SELECT id FROM t_money_accounts WHERE fiat_source = $1 AND currency = $2",
            "exchange",
            "USD"
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        // Setup: Give user initial balance of 100,000 USD
        sqlx::query!(
            r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
            usd_account_id,
            exchange_usd_account_id,
            "USD",
            Decimal::from(100_000),
            "test_deposit",
            uuid::Uuid::new_v4().to_string()
        )
        .execute(&pg_pool)
        .await
        .unwrap();

        // Test: Calculate initial balance
        let mut executor = pg_pool.acquire().await.unwrap();
        let balance = calculate_balance_from_user_and_currency(
            &mut executor,
            user_id.as_str(),
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap()
        .expect("User should have a balance after deposit");

        // Verify initial balance is 100,000 USD
        assert_eq!(
            balance.0,
            Decimal::from(100_000),
            "Initial balance should be 100,000 USD"
        );

        // Test: Reserve 100 USD
        let quantity = Money38_18(Decimal::from(100));
        let reserve_money = reserve_money_by_asset(
            &mut executor,
            user_id.as_str(),
            quantity,
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap();

        // Verify the ReserveMoney result
        assert_eq!(
            reserve_money.previous_balance,
            Decimal::from(100_000),
            "Previous balance should be 100,000 USD"
        );
        assert_eq!(
            reserve_money.new_balance,
            Some(Decimal::from(99_900)),
            "New balance should be 99,900 USD (100,000 - 100)"
        );

        // Verify actual database balance matches
        let db_balance = calculate_balance_from_user_and_currency(
            &mut executor,
            user_id.as_str(),
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap()
        .expect("User should still have a balance");

        assert_eq!(
            db_balance.0,
            Decimal::from(99_900),
            "Database balance should be 99,900 USD after reservation"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_revert_reservation(pg_pool: sqlx::PgPool) {
        // Setup: Create test user
        let user_tag = Uuid::new_v4();
        let clerk_id = format!("revert-clerk-{user_tag}");
        let name = format!("Test Revert User {user_tag}");
        let email = format!("test-revert-{user_tag}@example.com");

        let user_data_id = sqlx::query_scalar!(
            "INSERT INTO t_user_data (clerk, tier, name, email, password_hash) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            clerk_id,
            1,
            name,
            email,
            &[] as &[u8]
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        let user_id = user_data_id.to_string();

        // Setup: Create symbol vocabulary and currency
        let symbol_vocabulary = SymbolVocabulary::from_iter(vec!["USD".into()]);
        let currency = AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap();

        // Setup: Create user USD account
        let usd_account_id = sqlx::query_scalar!(
            "INSERT INTO t_money_accounts (currency, user_id) VALUES ($1, $2) RETURNING id",
            "USD",
            user_data_id
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        // Setup: Get exchange USD account (from migration)
        let exchange_usd_account_id = sqlx::query_scalar!(
            "SELECT id FROM t_money_accounts WHERE fiat_source = $1 AND currency = $2",
            "exchange",
            "USD"
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        // Setup: Give user initial balance of 100,000 USD
        sqlx::query!(
            r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
            usd_account_id,
            exchange_usd_account_id,
            "USD",
            Decimal::from(100_000),
            "test_deposit",
            uuid::Uuid::new_v4().to_string()
        )
        .execute(&pg_pool)
        .await
        .unwrap();

        // Test: Reserve 100 USD
        let mut executor = pg_pool.acquire().await.unwrap();
        let quantity = Money38_18(Decimal::from(100));
        let reserve_money = reserve_money_by_asset(
            &mut executor,
            user_id.as_str(),
            quantity,
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap();

        // Verify balance after reservation
        let balance_after_reserve = calculate_balance_from_user_and_currency(
            &mut executor,
            user_id.as_str(),
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap()
        .expect("User should have a balance");

        assert_eq!(
            balance_after_reserve.0,
            Decimal::from(99_900),
            "Balance should be 99,900 USD after reserving 100 USD"
        );

        // Store reservation transaction ID for later verification
        let reservation_tx_id = reserve_money.row_id as i32;

        // Test: Revert the reservation
        let revert_tx_id = reserve_money.revert(&pg_pool).await.unwrap();

        // Verify balance is restored after revert
        let balance_after_revert = calculate_balance_from_user_and_currency(
            &mut executor,
            user_id.as_str(),
            &currency,
            &symbol_vocabulary,
        )
        .await
        .unwrap()
        .expect("User should have a balance");

        assert_eq!(
            balance_after_revert.0,
            Decimal::from(100_000),
            "Balance should be restored to 100,000 USD after revert"
        );

        // Verify transaction journal: Fetch original reservation transaction
        let reservation_tx = sqlx::query!(
            r#"
            SELECT id, credit_account_id, debit_account_id, currency, amount, transaction_type
            FROM t_account_tx_journal
            WHERE id = $1
            "#,
            reservation_tx_id
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        assert_eq!(
            reservation_tx.transaction_type, "reserve asset",
            "Original transaction should have type 'reserve asset'"
        );
        assert_eq!(
            reservation_tx.credit_account_id, exchange_usd_account_id,
            "Original transaction should credit exchange account"
        );
        assert_eq!(
            reservation_tx.debit_account_id, usd_account_id,
            "Original transaction should debit user account"
        );
        assert_eq!(
            reservation_tx.amount,
            Decimal::from(100),
            "Original transaction amount should be 100"
        );

        // Verify transaction journal: Fetch revert transaction
        let revert_tx = sqlx::query!(
            r#"
            SELECT id, credit_account_id, debit_account_id, currency, amount, transaction_type
            FROM t_account_tx_journal
            WHERE id = $1
            "#,
            revert_tx_id
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        assert_eq!(
            revert_tx.transaction_type, "revert reserve asset",
            "Revert transaction should have type 'revert reserve asset'"
        );

        // Verify accounts are swapped
        assert_eq!(
            revert_tx.credit_account_id, reservation_tx.debit_account_id,
            "Revert should credit what reservation debited (user account)"
        );
        assert_eq!(
            revert_tx.debit_account_id, reservation_tx.credit_account_id,
            "Revert should debit what reservation credited (exchange account)"
        );

        // Verify amount and currency are preserved
        assert_eq!(
            revert_tx.amount, reservation_tx.amount,
            "Revert amount should match reservation amount"
        );
        assert_eq!(
            revert_tx.currency, reservation_tx.currency,
            "Revert currency should match reservation currency"
        );
    }
}
