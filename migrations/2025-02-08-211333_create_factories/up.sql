-- Your SQL goes here
CREATE TABLE factories (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    address VARCHAR NOT NULL UNIQUE,
    fee INTEGER NOT NULL,
    version VARCHAR NOT NULL
);

-- Index for address lookups
CREATE INDEX idx_factories_address ON factories(address);