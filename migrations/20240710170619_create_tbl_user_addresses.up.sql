CREATE TABLE IF NOT EXISTS user_addresses (
    id BIGSERIAL PRIMARY KEY,
    address_text TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    kind VARCHAR(10) CHECK (kind IN ('deposit', 'withdrawal')) NOT NULL,
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3,}$'),  -- Currency codes in uppercase
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(address_text, user_id, currency)
);
