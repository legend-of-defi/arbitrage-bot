-- This file should undo anything in `up.sql`

-- Create a new enum type without the 'Broken' value
CREATE TYPE factory_status_new AS ENUM ('Unsynced', 'Syncing', 'Synced');

-- First, drop the default constraint
ALTER TABLE factories ALTER COLUMN status DROP DEFAULT;

-- Update the column to use the new type
ALTER TABLE factories
  ALTER COLUMN status TYPE factory_status_new
  USING (
    CASE
      WHEN status::text = 'Broken' THEN 'Unsynced'::factory_status_new
      ELSE status::text::factory_status_new
    END
  );

-- Re-add the default constraint with the new type
ALTER TABLE factories ALTER COLUMN status SET DEFAULT 'Unsynced'::factory_status_new;

-- Drop the old type
DROP TYPE factory_status;

-- Rename the new type to the original name
ALTER TYPE factory_status_new RENAME TO factory_status;
