-- Drop indices
DROP INDEX IF EXISTS idx_pairs_factory;
DROP INDEX IF EXISTS idx_pairs_token0;
DROP INDEX IF EXISTS idx_pairs_token1;
DROP INDEX IF EXISTS idx_pairs_address;

-- Drop foreign key constraints
ALTER TABLE pairs
DROP CONSTRAINT IF EXISTS fk_token0,
DROP CONSTRAINT IF EXISTS fk_token1,
DROP CONSTRAINT IF EXISTS fk_factory;

-- Rename table back
ALTER TABLE pairs RENAME TO uniswap_pairs;

-- Add back constraints for old table name
ALTER TABLE uniswap_pairs
ADD CONSTRAINT fk_token0
FOREIGN KEY (token0_id) 
REFERENCES tokens(id);

ALTER TABLE uniswap_pairs
ADD CONSTRAINT fk_token1
FOREIGN KEY (token1_id) 
REFERENCES tokens(id);

ALTER TABLE uniswap_pairs
ADD CONSTRAINT fk_factory
FOREIGN KEY (factory_id) 
REFERENCES factories(id);

-- Recreate indices for old table name
CREATE INDEX idx_pairs_factory ON uniswap_pairs(factory_id);
CREATE INDEX idx_pairs_token0 ON uniswap_pairs(token0_id);
CREATE INDEX idx_pairs_token1 ON uniswap_pairs(token1_id);
CREATE INDEX idx_pairs_address ON uniswap_pairs(address); 