-- Add status column to factories table
-- Syncing... - when the factory is still syncing its pairs (allPairs function)
-- Synced - when the factory has synced all pairs and we rely on PairCreated event

CREATE TYPE factory_status AS ENUM ('Unsynced', 'Syncing...', 'Synced');
ALTER TABLE factories ADD COLUMN status factory_status NOT NULL DEFAULT 'Unsynced';

ALTER TABLE factories ALTER COLUMN last_pair_id SET DEFAULT 0;
UPDATE factories SET last_pair_id = 0 WHERE last_pair_id IS NULL;
ALTER TABLE factories ALTER COLUMN last_pair_id SET NOT NULL;


