-- Down migration for api_keys and api_key_audit_log tables, types

-- Drop the api_key_audit_log table
DROP TABLE IF EXISTS api_key_audit_log;

-- Drop the api_keys table
DROP TABLE IF EXISTS api_keys;

-- Drop the api_key_action type
DROP TYPE IF EXISTS api_key_action;
