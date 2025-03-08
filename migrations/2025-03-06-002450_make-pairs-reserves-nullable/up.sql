-- Your SQL goes here

-- Make reserve columns nullable
ALTER TABLE pairs
ALTER COLUMN reserve0 DROP NOT NULL,
ALTER COLUMN reserve1 DROP NOT NULL,
ALTER COLUMN usd DROP NOT NULL,
ALTER COLUMN token0_id DROP NOT NULL,
ALTER COLUMN token1_id DROP NOT NULL,
ALTER COLUMN factory_id DROP NOT NULL;

-- Set existing values to NULL
UPDATE pairs
SET reserve0 = NULL, reserve1 = NULL, usd = NULL;
