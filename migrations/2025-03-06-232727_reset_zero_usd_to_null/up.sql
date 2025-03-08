-- Your SQL goes here

-- Convert pairs.usd values of 0 to NULL
UPDATE pairs
SET usd = NULL
WHERE usd = 0;
