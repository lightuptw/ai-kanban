ALTER TABLE boards ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
UPDATE boards SET position = rowid * 1000;
