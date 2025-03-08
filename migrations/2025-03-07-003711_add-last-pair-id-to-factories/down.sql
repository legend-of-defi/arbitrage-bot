-- This file should undo anything in `up.sql`
ALTER TABLE factories
ADD COLUMN name TEXT,
ADD COLUMN fee INTEGER,
ADD COLUMN version TEXT,
DROP COLUMN last_pair_id;
