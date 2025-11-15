-- Insert an account for the exchange's cash account
INSERT INTO t_money_accounts (currency, fiat_source) VALUES ('USD', 'exchange');

-- Insert bitcoin account
INSERT INTO t_money_accounts (currency, crypto_source) VALUES ('BTC', 'bitcoin');

-- Insert BTC/USD trading pair
INSERT INTO t_trading_asset_pairs (
    base_asset, quote_asset, status,
    min_order_size, max_order_size,
    price_tick_size, quantity_tick_size,
    altname, wsname,
    pair_decimals, cost_decimals, lot_decimals,
    lot, lot_multiplier,
    aclass_base, aclass_quote,
    leverage_buy, leverage_sell,
    fees, fees_maker,
    fee_volume_currency,
    margin_call, margin_stop,
    costmin, tick_size,
    long_position_limit, short_position_limit
) VALUES (
    'BTC', 'ZUSD', 'active',
    0.0001, 100, 0.01, 0.0001,
    'BTCUSD', 'BTC/USD',
    1, 5, 8,
    'unit', 1,
    'currency', 'currency',
    '{2,3,4,5}', '{2,3,4,5}',
    '[[0,0.4],[10000,0.35],[50000,0.24],[100000,0.22],[250000,0.2],[500000,0.18],[1000000,0.16],[2500000,0.14],[5000000,0.12],[10000000,0.1]]'::jsonb,
    '[[0,0.25],[10000,0.2],[50000,0.14],[100000,0.12],[250000,0.1],[500000,0.08],[1000000,0.06],[2500000,0.04],[5000000,0.02],[10000000,0.0]]'::jsonb,
    'ZUSD',
    80, 40,
    0.5, 0.01,
    NULL, NULL
);
