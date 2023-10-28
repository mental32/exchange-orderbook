-- Drop the tables
DROP TABLE IF EXISTS account_tx_journal;
DROP TABLE IF EXISTS accounts;
DROP TABLE IF EXISTS account_sources;

-- Drop the triggers and functions
DROP TRIGGER IF EXISTS update_timestamp ON accounts;
DROP TRIGGER IF EXISTS validate_transaction ON tx_journal;
DROP FUNCTION IF EXISTS update_timestamp();
DROP FUNCTION IF EXISTS validate_transaction();
