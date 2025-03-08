-- This file should undo anything in `up.sql`
ALTER TABLE tokens
ALTER COLUMN decimals SET NOT NULL;
