-- Trading pairs configuration table
CREATE TABLE trading_pairs (
    id SERIAL PRIMARY KEY,
    -- Asset pair definition (using TEXT like accounts table)
    base_asset TEXT NOT NULL CHECK (base_asset ~ '^[A-Z]{3,7}$'),
    quote_asset TEXT NOT NULL CHECK (quote_asset ~ '^[A-Z]{3,7}$'),
    -- Operational status
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    -- Order size constraints (in base asset units)
    min_order_size DECIMAL(20, 8) NOT NULL CHECK (min_order_size > 0),
    max_order_size DECIMAL(20, 8) CHECK (max_order_size IS NULL OR max_order_size >= min_order_size),
    -- Tick size configuration (minimum increments)
    price_tick_size DECIMAL(20, 8) NOT NULL CHECK (price_tick_size > 0),
    quantity_tick_size DECIMAL(20, 8) NOT NULL CHECK (quantity_tick_size > 0),
    -- Audit
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Constraints
    UNIQUE (base_asset, quote_asset),
    CHECK (base_asset != quote_asset),
    CHECK (quantity_tick_size <= min_order_size)
);

-- Use existing update trigger function
SELECT manage_updated_at('trading_pairs');

-- Documentation
COMMENT ON TABLE trading_pairs IS 'Configuration for tradeable asset pairs in the matching engine using DECIMAL types for precise financial calculations';
COMMENT ON COLUMN trading_pairs.price_tick_size IS 'Minimum price increment for orders in quote asset units (e.g., $0.01 for USD pairs)';
COMMENT ON COLUMN trading_pairs.quantity_tick_size IS 'Minimum quantity increment for orders in base asset units (e.g., 0.00001 BTC)';
