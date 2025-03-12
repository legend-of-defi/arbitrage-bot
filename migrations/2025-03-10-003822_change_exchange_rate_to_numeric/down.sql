-- This file should undo anything in `up.sql`

-- Change the exchange_rate column type back from Numeric to Float8 (double precision)

-- First drop the index
DROP INDEX tokens_exchange_rate_idx;

-- Then alter the column type back
ALTER TABLE tokens 
  ALTER COLUMN exchange_rate TYPE FLOAT8 USING exchange_rate::FLOAT8;

-- Recreate the index
CREATE INDEX tokens_exchange_rate_idx ON tokens(exchange_rate);

-- Revert the column comment
COMMENT ON COLUMN tokens.exchange_rate IS 'Exchange rate of the token in USD';
