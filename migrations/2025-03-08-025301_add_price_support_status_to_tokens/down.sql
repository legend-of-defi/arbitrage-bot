-- This file should undo anything in `up.sql`

-- Drop the price_support_status column from the tokens table
ALTER TABLE tokens
DROP COLUMN price_support_status;

-- Drop the price_support_status enum type
DROP TYPE price_support_status;
