-- This file should undo anything in `up.sql`
ALTER TABLE tokens
DROP COLUMN exchange_rate,
DROP COLUMN updated_last;

-- Drop the indexes
DROP INDEX tokens_exchange_rate_idx;
DROP INDEX tokens_updated_last_idx;