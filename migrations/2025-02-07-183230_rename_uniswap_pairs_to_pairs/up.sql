-- Drop existing foreign key constraints first
ALTER TABLE uniswap_pairs
DROP CONSTRAINT IF EXISTS fk_token0,
DROP CONSTRAINT IF EXISTS fk_token1,
DROP CONSTRAINT IF EXISTS fk_factory;

-- Drop existing indices
DROP INDEX IF EXISTS idx_pairs_factory;
DROP INDEX IF EXISTS idx_pairs_token0;
DROP INDEX IF EXISTS idx_pairs_token1;
DROP INDEX IF EXISTS idx_pairs_address;

-- Rename the table
ALTER TABLE uniswap_pairs RENAME TO pairs;

-- Add back foreign key constraints with new table name
ALTER TABLE pairs
ADD CONSTRAINT fk_token0
FOREIGN KEY (token0_id) 
REFERENCES tokens(id);

ALTER TABLE pairs
ADD CONSTRAINT fk_token1
FOREIGN KEY (token1_id) 
REFERENCES tokens(id);

ALTER TABLE pairs
ADD CONSTRAINT fk_factory
FOREIGN KEY (factory_id) 
REFERENCES factories(id);

-- Create new indices with new table name
CREATE INDEX idx_pairs_factory ON pairs(factory_id);
CREATE INDEX idx_pairs_token0 ON pairs(token0_id);
CREATE INDEX idx_pairs_token1 ON pairs(token1_id);
CREATE INDEX idx_pairs_address ON pairs(address); 