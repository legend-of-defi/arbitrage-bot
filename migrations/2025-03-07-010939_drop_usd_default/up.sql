-- Your SQL goes here

-- Drop the default value for the usd column
ALTER TABLE pairs ALTER COLUMN usd DROP DEFAULT;

-- Set any remaining USD=0 values to NULL
UPDATE pairs SET usd = NULL WHERE usd = 0;
