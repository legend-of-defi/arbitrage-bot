-- Your SQL goes here
CREATE TABLE pairs (
    id SERIAL PRIMARY KEY,
    address VARCHAR NOT NULL UNIQUE,
    token0_id INTEGER NOT NULL REFERENCES tokens(id),
    token1_id INTEGER NOT NULL REFERENCES tokens(id),
    factory_id INTEGER NOT NULL REFERENCES factories(id),
    reserve0 NUMERIC NOT NULL DEFAULT 0,
    reserve1 NUMERIC NOT NULL DEFAULT 0,
    usd INTEGER NOT NULL DEFAULT 0
);

-- Index for factory relationship
CREATE INDEX idx_pairs_factory ON pairs(factory_id);

-- Indices for token relationships
CREATE INDEX idx_pairs_token0 ON pairs(token0_id);
CREATE INDEX idx_pairs_token1 ON pairs(token1_id);

-- Index for address lookups
CREATE INDEX idx_pairs_address ON pairs(address);