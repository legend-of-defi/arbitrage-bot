-- Your SQL goes here

-- Create the price support status enum type
CREATE TYPE price_support_status AS ENUM (
    'SUPPORTED',     -- Price data is available for this token
    'UNSUPPORTED'    -- Price data is not available for this token
);

-- Add the price_support_status column to the tokens table
ALTER TABLE tokens
ADD COLUMN price_support_status price_support_status DEFAULT NULL;

-- Add a comment to explain the column usage
COMMENT ON COLUMN tokens.price_support_status IS 'Indicates whether price data is available for this token from external APIs. NULL means not yet checked.';
