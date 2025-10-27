-- Enable UUID generation extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE DOMAIN money38_18 AS DECIMAL(38, 18)
  DEFAULT 0
  CHECK (VALUE >= 0);


-- MARK: FUNCTIONS

-- Set the `updated_at` column to the current timestamp if the row was modified
--
CREATE OR REPLACE FUNCTION f_set_updated_at()
    RETURNS TRIGGER
    AS $$
BEGIN
    IF(NEW IS DISTINCT FROM OLD AND NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at) THEN
        NEW.updated_at := CURRENT_TIMESTAMP;
    END IF;
    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

-- Validate a transaction before inserting into transaction journal
--
CREATE OR REPLACE FUNCTION f_validate_transaction()
    RETURNS TRIGGER
    AS $$
BEGIN
    IF NEW.amount <= 0 THEN
        RAISE EXCEPTION 'Transaction amount must be greater than zero.';
    END IF;
    IF NEW.credit_account_id = NEW.debit_account_id THEN
        RAISE EXCEPTION 'Credit and Debit accounts must be different.';
    END IF;
    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

-- reject some operation with a reason message
--
-- Ensure any previous incompatible overloads are removed before creating the trigger function
DROP FUNCTION IF EXISTS f_reject(TEXT) CASCADE;
DROP FUNCTION IF EXISTS f_reject() CASCADE;

CREATE OR REPLACE FUNCTION f_reject()
    RETURNS TRIGGER
    AS $$
BEGIN
    -- Trigger arguments are available via TG_ARGV array; use first arg as reason
    RAISE EXCEPTION 'trigger rejected operation reason=%', TG_ARGV[0];
END;
$$
LANGUAGE plpgsql;

-- is valid currency code
--
CREATE OR REPLACE FUNCTION f_valid_currency_code(code TEXT)
RETURNS BOOLEAN AS $$
BEGIN
    RETURN code ~ '^[A-Z]{3,7}$'; -- lower bound is 3 (ISO 4217), upper bound is 7 so we can have a size for `AssetCode` in Rust (array of 8 bytes)
END;
$$ LANGUAGE plpgsql;

-- Calculate the balance for a given user and currency
--
CREATE OR REPLACE FUNCTION f_calculate_balance(source_uuid TEXT, currency_code TEXT)
RETURNS money38_18 AS $$
DECLARE
    account_id INT;
    credit_total money38_18;
    debit_total money38_18;
BEGIN
    -- Attempt to retrieve the account_id
    SELECT id INTO account_id FROM t_money_accounts
    WHERE user_id = source_uuid::integer AND currency = currency_code;

    -- Check if the account_id was found
    IF account_id IS NULL THEN
        RAISE EXCEPTION 'No account found for the given user ID and currency.';
    END IF;

    -- Calculate the total credits
    SELECT COALESCE(SUM(amount), 0) INTO credit_total
    FROM t_account_tx_journal WHERE credit_account_id = account_id;

    -- Calculate the total debits
    SELECT COALESCE(SUM(amount), 0) INTO debit_total
    FROM t_account_tx_journal WHERE debit_account_id = account_id;

    -- Return the balance
    RETURN credit_total - debit_total;
END;
$$ LANGUAGE plpgsql;

-- MARK: USERS

DO $$
BEGIN
    CREATE TYPE user_role AS ENUM(
        'user',
        'admin',
        'oper'
);
EXCEPTION
    WHEN duplicate_object THEN
        NULL;
END
$$;

CREATE TABLE IF NOT EXISTS t_user_data(
    id serial PRIMARY KEY,
    clerk text UNIQUE,
    tier integer NOT NULL CHECK (tier >= 1 AND tier <= 3),
    name varchar(255) NOT NULL,
    email varchar(255) NOT NULL UNIQUE,
    password_hash bytea NOT NULL, -- Using bytea for hash
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at timestamptz,
    user_role user_role NOT NULL DEFAULT 'user'
);

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON t_user_data
    FOR EACH ROW
    EXECUTE FUNCTION f_set_updated_at(); -- Trigger to update 'updated_at' timestamp on record update in 't_user_data'

-- MARK: WALLETS

-- User addresses table
CREATE TABLE IF NOT EXISTS t_user_addresses (
    id              bigserial PRIMARY KEY,                                          -- Using bigserial for potentially large number of addresses
    user_id         integer   NOT NULL REFERENCES t_user_data(id),                  -- Foreign key to users table
    address_text    text      NOT NULL,                                             -- human-readable wallet address i.e. "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
    kind            varchar(10) CHECK (kind IN ('deposit', 'withdrawal')) NOT NULL, -- Type of address: deposit means funds arrive to here, withdrawal means funds may leave to here
    currency        text      NOT NULL CHECK (f_valid_currency_code(currency)),     -- Currency codes
    created_at      timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,                   -- Timestamp of creation
    updated_at      timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,                   -- Timestamp of last update
    UNIQUE (address_text, user_id, currency)                                        -- unique address per user and currency
);

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON t_user_addresses
    FOR EACH ROW
    EXECUTE FUNCTION f_set_updated_at(); -- Trigger to update 'updated_at' timestamp on record update in 't_user_addresses'


-- MARK: ACCOUNTING
-- accounts table tracks all accounts in the system that can interact with money
--
-- sometimes accounts are not users! for example, a chain deposit or withdrawal account, or a bank transfer
-- this is why we have account-source, to track the source of the account and that will tell us what type of account it is
--
CREATE TABLE IF NOT EXISTS t_money_accounts(
    id serial PRIMARY KEY,
    currency text NOT NULL CHECK (f_valid_currency_code(currency)), -- Currency codes in uppercase
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Exactly one of these must be non-NULL
    user_id integer NULL REFERENCES t_user_data(id) ON DELETE RESTRICT,
    fiat_source text NULL,
    crypto_source text NULL,

    -- Ensure exactly one source type is populated
    CHECK (
        (user_id IS NOT NULL)::int +
        (fiat_source IS NOT NULL)::int +
        (crypto_source IS NOT NULL)::int = 1
    ),

    -- Unique constraints per account type
    UNIQUE (user_id, currency),
    UNIQUE (fiat_source, currency),
    UNIQUE (crypto_source, currency)
);

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON t_money_accounts
    FOR EACH ROW
    EXECUTE FUNCTION f_set_updated_at(); -- Trigger to update 'updated_at' timestamp on record update in 't_money_accounts'

-- A crude double-entry accounting journal, it is used to keep track of all transactions between accounts
--
-- each row tracks a single transaction between two accounts, the credit and debit accounts
-- the credit account is the account that is receiving the funds, the debit account is the account that is sending the funds
--
-- transaction_type is used to track the source of the transaction, for example, a user deposit, a user withdrawal, a chain deposit, a chain withdrawal, etc.
--
-- amount is always a positive number, the credit account is increased by the amount, the debit account is decreased by the amount
--
-- currency is a "ISO 4217"-like currency code in uppercase, or in the case of crypto, the unofficial currency symbol in uppercase
--
CREATE TABLE IF NOT EXISTS t_account_tx_journal(
    id serial PRIMARY KEY,
    credit_account_id int NOT NULL,
    debit_account_id int NOT NULL,
    currency text NOT NULL CHECK (f_valid_currency_code(currency)), -- Currency codes in uppercase
    amount money38_18 NOT NULL CHECK (amount > 0), -- Using money38_18 domain to ensure non-negative amounts with high precision
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    transaction_type text NOT NULL,
    txid text NOT NULL,
    FOREIGN KEY (credit_account_id) REFERENCES t_money_accounts(id),
    FOREIGN KEY (debit_account_id) REFERENCES t_money_accounts(id),
    CHECK (credit_account_id != debit_account_id),
    CHECK (txid != '')
    , UNIQUE (txid)
);


-- Trigger to validate transactions before inserting into t_account_tx_journal
CREATE TRIGGER f_validate_transaction
    BEFORE INSERT ON t_account_tx_journal
    FOR EACH ROW
    EXECUTE FUNCTION f_validate_transaction();

-- Outbox table for reliable event publishing from the accounting journal
-- Implements the transactional outbox pattern to ensure atomicity between
-- database writes and event notifications
CREATE TABLE IF NOT EXISTS t_account_tx_journal_outbox(
    id bigserial PRIMARY KEY,
    journal_id int NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at timestamptz,
    FOREIGN KEY (journal_id) REFERENCES t_account_tx_journal(id) ON DELETE CASCADE
);

-- Trigger function to populate outbox and notify listeners when journal entries are created
CREATE OR REPLACE FUNCTION f_journal_to_outbox()
    RETURNS TRIGGER
    AS $$
BEGIN
    -- Insert reference to the new journal entry into the outbox
    INSERT INTO t_account_tx_journal_outbox(journal_id)
    VALUES (NEW.id);

    -- Notify listeners with JSON payload containing journal_id and txid for efficient processing
    PERFORM pg_notify(
        'account_tx_journal',
        json_build_object(
            'journal_id', NEW.id,
            'txid', NEW.txid
        )::text
    );

    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

-- Trigger to populate outbox after successful journal insert
CREATE TRIGGER tr_journal_outbox
    AFTER INSERT ON t_account_tx_journal
    FOR EACH ROW
    EXECUTE FUNCTION f_journal_to_outbox();


-- MARK: TRADING

-- append-only table for event sourcing of input messages to asset-pair processors in the matching engine
--
-- Event Sourcing: https://microservices.io/patterns/data/event-sourcing.html https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing
CREATE TABLE IF NOT EXISTS t_trading_event_source(
    id bigserial PRIMARY KEY,
    jstr jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    base_asset text NOT NULL,
    quote_asset text NOT NULL
);

-- reject modifications to t_trading_event_source (append-only)
CREATE TRIGGER tr_reject_update
    BEFORE UPDATE ON t_trading_event_source
    FOR EACH ROW
    EXECUTE FUNCTION f_reject('table is append-only');

CREATE TRIGGER tr_reject_delete
    BEFORE DELETE ON t_trading_event_source
    FOR EACH ROW
    EXECUTE FUNCTION f_reject('table is append-only');

-- Trading pairs configuration table
CREATE TABLE t_trading_asset_pairs( -- Configuration for tradeable asset pairs in the matching engine using DECIMAL types for precise financial calculations
    id serial PRIMARY KEY,
    base_asset text NOT NULL CHECK (f_valid_currency_code(base_asset)), -- Base asset code (e.g., BTC in BTC/USD)
    quote_asset text NOT NULL CHECK (f_valid_currency_code(quote_asset)), -- Quote asset code (e.g., USD in BTC/USD)
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')), -- Status of the trading pair
    min_order_size money38_18 NOT NULL CHECK (min_order_size > 0), -- Minimum order size in base asset units (e.g., 0.0001 BTC)
    max_order_size money38_18 NOT NULL CHECK (max_order_size > 0 AND max_order_size >= min_order_size), -- Maximum order size in base asset units (e.g., 100 BTC)
    price_tick_size money38_18 NOT NULL CHECK (price_tick_size > 0), -- Minimum price increment for orders in quote asset units (e.g., 0.01 USD)
    quantity_tick_size money38_18 NOT NULL CHECK (quantity_tick_size > 0 AND quantity_tick_size <= min_order_size), -- Minimum quantity increment for orders in base asset units (e.g., 0.0001 BTC)
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (base_asset, quote_asset),
    CHECK (base_asset != quote_asset),
    CHECK (quantity_tick_size <= min_order_size)
);

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON t_trading_asset_pairs
    FOR EACH ROW
    EXECUTE FUNCTION f_set_updated_at();

-- Add foreign key constraint to ensure event source records reference valid trading pairs
ALTER TABLE t_trading_event_source
    ADD CONSTRAINT fk_trading_pair
    FOREIGN KEY (base_asset, quote_asset)
    REFERENCES t_trading_asset_pairs(base_asset, quote_asset);
