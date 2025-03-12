-- Add exchange_rate and updated_last columns to tokens table
ALTER TABLE tokens
ADD COLUMN exchange_rate FLOAT,
ADD COLUMN updated_last TIMESTAMP;

-- Add indexes for exchange_rate and updated_last columns
CREATE INDEX tokens_exchange_rate_idx ON tokens(exchange_rate);
CREATE INDEX tokens_updated_last_idx ON tokens(updated_last);

-- Add comments explaining the columns
COMMENT ON COLUMN tokens.exchange_rate IS 'Exchange rate of the token in USD';
COMMENT ON COLUMN tokens.updated_last IS 'Timestamp of when the exchange rate was last updated';