-- orders_event_source table
CREATE TABLE IF NOT EXISTS orders_event_source (
    id BIGSERIAL PRIMARY KEY,
    jstr JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);


-- create a trigger on update to reject modifications to the rows
-- this is a append-only table
CREATE OR REPLACE FUNCTION reject_update()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'orders_event_source is an append-only table with immutable rows.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER reject_update
BEFORE UPDATE ON orders_event_source
FOR EACH ROW EXECUTE FUNCTION reject_update();
