-- Insert an account for the exchange's cash account
INSERT INTO t_money_accounts (currency, fiat_source) VALUES ('USD', 'exchange');

-- Insert bitcoin account
INSERT INTO t_money_accounts (currency, crypto_source) VALUES ('BTC', 'bitcoin');

-- Insert BTC/USD trading pair
INSERT INTO t_trading_asset_pairs (
    base_asset, quote_asset, status,
    min_order_size, max_order_size,
    price_tick_size, quantity_tick_size
) VALUES (
    'BTC', 'USD', 'active',
    0.0001, 100, 0.01, 0.0001
);
