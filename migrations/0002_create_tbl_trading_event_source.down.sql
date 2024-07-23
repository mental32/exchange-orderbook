-- Down migration for orders_event_source table and trigger

-- Drop the trigger
DROP TRIGGER IF EXISTS reject_update ON orders_event_source;

-- Drop the trigger function
DROP FUNCTION IF EXISTS reject_update;

-- Drop the table
DROP TABLE IF EXISTS orders_event_source;
