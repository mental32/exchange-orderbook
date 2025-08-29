-- Insert an account for the exchange's cash account
INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ('USD', 'fiat', 'fiat:exchange');

-- Insert bitcoin account
INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ('BTC', 'crypto', 'crypto:bitcoin');
