-- This file should undo anything in `up.sql`

ALTER TABLE factories DROP COLUMN status;
DROP TYPE factory_status;
ALTER TABLE factories ALTER COLUMN last_pair_id DROP NOT NULL;
ALTER TABLE factories ALTER COLUMN last_pair_id DROP DEFAULT;
