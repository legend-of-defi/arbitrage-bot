-- Your SQL goes here

-- Change the exchange_rate column type from Float8 (double precision) to Numeric

-- First drop the index
DROP INDEX tokens_exchange_rate_idx;

-- Then alter the column type
ALTER TABLE tokens 
  ALTER COLUMN exchange_rate TYPE NUMERIC USING exchange_rate::NUMERIC;

-- Recreate the index
CREATE INDEX tokens_exchange_rate_idx ON tokens(exchange_rate);

-- Update the column comment to reflect precision change
COMMENT ON COLUMN tokens.exchange_rate IS 'Exchange rate of the token in USD using Numeric for precise decimal representation';
