-- There is a separate worker that fills the decimals for tokens.
ALTER TABLE tokens
ALTER COLUMN decimals DROP NOT NULL;