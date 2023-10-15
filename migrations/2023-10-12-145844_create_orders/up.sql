CREATE TYPE IF NOT EXISTS order_type AS ENUM ('BUY', 'SELL');
CREATE TYPE IF NOT EXISTS order_status AS ENUM ('OPEN', 'CLOSED', 'CANCELLED');
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency_id INT NOT NULL REFERENCES currencies(id),
    type order_type NOT NULL,
    amount BIGINT NOT NULL CHECK (amount > 0),
    filled_amount BIGINT NOT NULL CHECK (filled_amount >= 0),
    remaining_amount BIGINT,
    price BIGINT NOT NULL CHECK (price > 0),
    status order_status NOT NULL DEFAULT 'OPEN',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);