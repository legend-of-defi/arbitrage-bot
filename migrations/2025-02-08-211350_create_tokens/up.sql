-- Your SQL goes here
CREATE TABLE tokens (
    id SERIAL PRIMARY KEY,
    address VARCHAR NOT NULL UNIQUE,
    symbol VARCHAR,
    name VARCHAR,
    decimals INTEGER NOT NULL,
    total_supply VARCHAR
);

-- Index for address lookups
CREATE UNIQUE INDEX idx_tokens_address ON tokens(address);