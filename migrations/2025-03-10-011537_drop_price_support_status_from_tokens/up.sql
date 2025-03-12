-- Your SQL goes here

-- Drop the column
ALTER TABLE tokens DROP COLUMN price_support_status;

-- Drop the custom type
DROP TYPE price_support_status;
