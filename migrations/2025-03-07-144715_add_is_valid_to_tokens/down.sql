-- This file should undo anything in `up.sql`

ALTER TABLE tokens DROP COLUMN is_valid;
