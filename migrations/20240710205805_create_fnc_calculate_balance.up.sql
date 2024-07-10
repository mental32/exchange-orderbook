CREATE OR REPLACE FUNCTION calculate_balance(source_uuid TEXT, currency_code TEXT)
RETURNS BIGINT AS $$
DECLARE
    account_id INT;
    credit_total BIGINT;
    debit_total BIGINT;
BEGIN
    -- Attempt to retrieve the account_id
    SELECT id INTO account_id FROM accounts 
    WHERE source_type = 'user' AND source_id = source_uuid AND currency = currency_code;

    -- Check if the account_id was found
    IF account_id IS NULL THEN
        RAISE EXCEPTION 'No account found for the given user ID and currency.';
    END IF;

    -- Calculate the total credits
    SELECT COALESCE(SUM(amount), 0) INTO credit_total
    FROM account_tx_journal WHERE credit_account_id = account_id;

    -- Calculate the total debits
    SELECT COALESCE(SUM(amount), 0) INTO debit_total
    FROM account_tx_journal WHERE debit_account_id = account_id;

    -- Return the balance
    RETURN credit_total - debit_total;
END;
$$ LANGUAGE plpgsql;
