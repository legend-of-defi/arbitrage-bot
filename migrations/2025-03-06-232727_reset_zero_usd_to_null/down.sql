-- This file should undo anything in `up.sql`

-- Convert pairs.usd NULL values back to 0
UPDATE pairs
SET usd = 0
WHERE usd IS NULL;
