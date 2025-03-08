

UPDATE pairs
SET reserve0 = '0', reserve1 = '0', usd = 0
WHERE reserve0 IS NULL OR reserve1 IS NULL OR usd IS NULL;

-- Make reserve columns NOT NULL again
ALTER TABLE pairs
ALTER COLUMN reserve0 SET NOT NULL,
ALTER COLUMN reserve1 SET NOT NULL,
ALTER COLUMN usd SET NOT NULL,
ALTER COLUMN token0_id SET NOT NULL,
ALTER COLUMN token1_id SET NOT NULL,
ALTER COLUMN factory_id SET NOT NULL;

