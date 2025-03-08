-- This file should undo anything in `up.sql`

-- Restore the default value for the usd column
ALTER TABLE pairs ALTER COLUMN usd SET DEFAULT 0;
