-- Your SQL goes here

DROP INDEX idx_factories_address;
CREATE UNIQUE INDEX idx_factories_address ON factories(address);

DROP INDEX idx_tokens_address;
CREATE UNIQUE INDEX idx_tokens_address ON tokens(address);

DROP INDEX idx_pairs_address;
CREATE UNIQUE INDEX idx_pairs_address ON pairs(address);
