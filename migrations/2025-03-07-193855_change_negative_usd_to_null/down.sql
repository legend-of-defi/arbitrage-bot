-- This file should undo anything in `up.sql`

-- Convert NULL usd values back to -1 (Note: This may affect usd values that were NULL for other reasons)
UPDATE pairs
SET usd = -1
WHERE usd IS NULL;
