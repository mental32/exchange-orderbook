CREATE INDEX idx_wallets_user_id ON wallets (user_id);
CREATE INDEX idx_orders_user_id ON orders (user_id);
CREATE INDEX idx_deposits_user_id ON deposits (user_id);
CREATE INDEX idx_withdrawals_user_id ON withdrawals (user_id);