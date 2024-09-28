BEGIN;

ALTER TABLE account_tx_journal
DROP CONSTRAINT chk_txid_not_empty;

DROP INDEX idx_account_tx_journal_txid;

ALTER TABLE account_tx_journal
DROP COLUMN txid;

COMMIT;
