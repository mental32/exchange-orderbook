-- Down migration for accounts and account_tx_journal tables, triggers, and functions

-- Drop the validate_transaction trigger
DROP TRIGGER IF EXISTS validate_transaction ON account_tx_journal;

-- Drop the validate_transaction function
DROP FUNCTION IF EXISTS validate_transaction;

-- Drop the update_timestamp trigger
DROP TRIGGER IF EXISTS update_timestamp ON accounts;

-- Drop the update_timestamp function
DROP FUNCTION IF EXISTS update_timestamp;

-- Drop the account_tx_journal table
DROP TABLE IF EXISTS account_tx_journal;

-- Drop the accounts table
DROP TABLE IF EXISTS accounts;
