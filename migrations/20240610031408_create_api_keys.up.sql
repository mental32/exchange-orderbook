CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE api_key_action AS ENUM ('created', 'accessed', 'revoked');

CREATE TABLE IF NOT EXISTS api_keys (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id UUID NOT NULL REFERENCES users(id),
    api_key BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    name VARCHAR(255),
    ip_restrictions TEXT[],
    scopes TEXT[],
    revoked_reason TEXT
);

CREATE TABLE IF NOT EXISTS api_key_audit_log (
    id SERIAL PRIMARY KEY,
    api_key_id INTEGER NOT NULL REFERENCES api_keys(id),
    accessed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action api_key_action NOT NULL,
    ip_address INET
);

-- Uncomment and run this query separately if needed:
-- SELECT api_key_id, MAX(accessed_at) AS last_used
-- FROM api_key_audit_log
-- WHERE action = 'accessed'
-- GROUP BY api_key_id;
