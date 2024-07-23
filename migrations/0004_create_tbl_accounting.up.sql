-- accounts table tracks all accounts in the system that can interact with money
--
-- sometimes accounts are not users! for example, a chain deposit or withdrawal account, or a bank transfer
-- this is why we have account-source, to track the source of the account and that will tell us what type of account it is
--
CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3,}$'),  -- Currency codes in uppercase
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    source_type TEXT NOT NULL CHECK (source_type IN ('user', 'fiat', 'crypto')),
    source_id TEXT NOT NULL, -- btc/eth if crypto, bank transfer id if fiat, user uuid if user
    UNIQUE (source_id, currency)
);

-- insert an account for the exchange's cash account
INSERT INTO accounts (currency, source_type, source_id) VALUES ('USD', 'fiat', 'exchange');

-- this table is a crude double-entry accounting journal, it is used to keep track of all transactions between accounts
--
-- each row tracks a single transaction between two accounts, the credit and debit accounts
-- the credit account is the account that is receiving the funds, the debit account is the account that is sending the funds
--
-- the transaction type is used to track the source of the transaction, for example, a user deposit, a user withdrawal, a chain deposit, a chain withdrawal, etc.
--
-- the amount is the amount of the transaction in the smallest unit of the currency, for example, satoshis for BTC, wei for ETH, cents for USD, etc.
--
-- the currency is a ISO 4217 currency code in uppercase, or in the case of crypto, the unofficial currency symbol in uppercase
--
CREATE TABLE IF NOT EXISTS account_tx_journal (
    id SERIAL PRIMARY KEY,
    credit_account_id INT NOT NULL,
    debit_account_id INT NOT NULL,
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3,}$'),
    amount BIGINT NOT NULL CHECK (amount >= 0),
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    transaction_type TEXT NOT NULL,
    FOREIGN KEY (credit_account_id) REFERENCES accounts(id),
    FOREIGN KEY (debit_account_id) REFERENCES accounts(id),
    CHECK (credit_account_id != debit_account_id)
);

-- Trigger to update 'updated_at' timestamp on record update in 'accounts'
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = current_timestamp;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_timestamp
BEFORE UPDATE ON accounts
FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- Function to validate a transaction before inserting into tx_journal
CREATE OR REPLACE FUNCTION validate_transaction()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.amount <= 0 THEN
        RAISE EXCEPTION 'Transaction amount must be greater than zero.';
    END IF;

    IF NEW.credit_account_id = NEW.debit_account_id THEN
        RAISE EXCEPTION 'Credit and Debit accounts must be different.';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER validate_transaction
BEFORE INSERT ON account_tx_journal
FOR EACH ROW EXECUTE FUNCTION validate_transaction();
