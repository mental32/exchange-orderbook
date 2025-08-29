-- Reverse the initial migration by dropping all created objects
-- Order matters: drop dependent objects first, then their dependencies

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS t_trading_asset_pairs;
DROP TABLE IF EXISTS t_trading_event_source;
DROP TABLE IF EXISTS t_account_tx_journal;
DROP TABLE IF EXISTS t_money_accounts;
DROP TABLE IF EXISTS t_user_addresses;
DROP TABLE IF EXISTS t_users;

-- Drop triggers first
DROP TRIGGER IF EXISTS f_validate_transaction ON account_tx_journal;
DROP TRIGGER IF EXISTS f_reject_update_append_only ON trading_event_source;

DROP TYPE IF EXISTS user_role;
DROP FUNCTION IF EXISTS f_reject_update_append_only();
DROP FUNCTION IF EXISTS f_validate_transaction();
DROP FUNCTION IF EXISTS f_set_updated_at();
DROP FUNCTION IF EXISTS f_manage_updated_at(_tbl regclass);

