-- This file should undo anything in `up.sql`

-- Re-create the custom type
CREATE TYPE price_support_status AS ENUM ('SUPPORTED', 'UNSUPPORTED');

-- Re-add the column
ALTER TABLE tokens ADD COLUMN price_support_status price_support_status;
