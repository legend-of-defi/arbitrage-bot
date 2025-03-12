-- Your SQL goes here

ALTER TABLE tokens ADD COLUMN is_valid BOOLEAN NOT NULL DEFAULT true;
