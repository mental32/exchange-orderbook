-- Reverse the initial migration by dropping all created objects
-- Order matters: drop dependent objects first, then their dependencies

DROP TABLE IF EXISTS t_trading_event_source;
DROP TABLE IF EXISTS t_trading_asset_pairs;
DROP TABLE IF EXISTS t_account_tx_journal_outbox;
DROP TABLE IF EXISTS t_account_tx_journal;
DROP TABLE IF EXISTS t_money_accounts;
DROP TABLE IF EXISTS t_user_addresses;
DROP TABLE IF EXISTS t_user_data;

DROP TRIGGER IF EXISTS tr_journal_outbox ON t_account_tx_journal;
DROP TRIGGER IF EXISTS f_validate_transaction ON account_tx_journal;
DROP TRIGGER IF EXISTS f_reject_update_append_only ON trading_event_source;

DROP TYPE IF EXISTS user_role;
DROP FUNCTION IF EXISTS f_journal_to_outbox();
DROP FUNCTION IF EXISTS f_reject_update_append_only();
DROP FUNCTION IF EXISTS f_validate_transaction();
DROP FUNCTION IF EXISTS f_set_updated_at();
DROP FUNCTION IF EXISTS f_manage_updated_at(_tbl regclass);
DROP FUNCTION IF EXISTS f_reject();
DROP FUNCTION IF EXISTS f_calculate_balance(source_uuid text, currency_code text);
DROP FUNCTION IF EXISTS f_valid_currency_code(code text);

DROP TYPE money38_18;
