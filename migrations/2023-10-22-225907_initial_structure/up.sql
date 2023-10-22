-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    -- Using bytea for hash
    password_hash BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    UNIQUE(email)
);

-- currencies table
CREATE TYPE currency_status AS ENUM ('ACTIVE', 'INACTIVE', 'DEPRECATED');
CREATE TABLE IF NOT EXISTS currencies (
    id SERIAL PRIMARY KEY,
    symbol CHAR(9) NOT NULL UNIQUE,
    name TEXT NOT NULL,
    status currency_status NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX idx_symbol ON currencies (symbol);

-- wallets table
CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency_id INT NOT NULL REFERENCES currencies(id),
    balance BIGINT NOT NULL CHECK (balance >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, currency_id)
);

-- orders_event_source table
CREATE TABLE IF NOT EXISTS orders_event_source (
    id BIGSERIAL PRIMARY KEY,
    jstr JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);


-- create a trigger on update to reject modifications to the rows
-- this is a append-only table
CREATE OR REPLACE FUNCTION reject_update()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'orders_event_source is an append-only table with immutable rows.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER reject_update
BEFORE UPDATE ON orders_event_source
FOR EACH ROW EXECUTE FUNCTION reject_update();

-- deposits and withdrawals tables
CREATE TYPE tx_status AS ENUM ('CREATED', 'PENDING', 'COMPLETED', 'FAILED');
CREATE TABLE IF NOT EXISTS deposits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency_id INT NOT NULL REFERENCES currencies(id),
    amount BIGINT NOT NULL CHECK (amount > 0),
    status tx_status NOT NULL DEFAULT 'CREATED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS withdrawals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency_id INT NOT NULL REFERENCES currencies(id),
    amount BIGINT NOT NULL CHECK (amount > 0),
    status tx_status NOT NULL DEFAULT 'CREATED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
