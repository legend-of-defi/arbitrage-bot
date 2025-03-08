-- This file should undo anything in `up.sql`

DROP INDEX idx_factories_address;
CREATE INDEX idx_factories_address ON factories(address);

DROP INDEX idx_tokens_address;
CREATE INDEX idx_tokens_address ON tokens(address);

DROP INDEX idx_pairs_address;
CREATE INDEX idx_pairs_address ON pairs(address);