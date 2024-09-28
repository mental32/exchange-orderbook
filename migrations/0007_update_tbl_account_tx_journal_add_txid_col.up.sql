BEGIN;

ALTER TABLE account_tx_journal
ADD COLUMN txid TEXT NOT NULL;

CREATE UNIQUE INDEX idx_account_tx_journal_txid ON account_tx_journal(txid);

ALTER TABLE account_tx_journal
ADD CONSTRAINT chk_txid_not_empty CHECK (txid != '');

COMMIT;
