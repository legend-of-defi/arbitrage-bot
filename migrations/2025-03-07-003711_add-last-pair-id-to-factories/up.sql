ALTER TABLE factories
-- This tracks the last pair id that was synced for a factory so we can resume from where we left off
ADD COLUMN last_pair_id INTEGER;

-- We can't pull these from the factory contract and we don't need them
ALTER TABLE factories
DROP COLUMN name,
DROP COLUMN fee,
DROP COLUMN version;
