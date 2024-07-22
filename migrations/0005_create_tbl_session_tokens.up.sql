CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS session_tokens (
    id SERIAL PRIMARY KEY,
    token BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    max_age INT NOT NULL DEFAULT 3600,
    user_id UUID NOT NULL REFERENCES users(id),
    ip_address VARCHAR(45), -- Supports both IPv4 and IPv6
    user_agent TEXT,
    last_accessed_at TIMESTAMP
);
