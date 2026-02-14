-- Add boards table for multi-board support
CREATE TABLE boards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Add board_id to cards table
ALTER TABLE cards ADD COLUMN board_id TEXT REFERENCES boards(id) ON DELETE CASCADE;

-- Create default board
INSERT INTO boards (id, name, created_at, updated_at) 
VALUES ('default', 'Main Board', datetime('now'), datetime('now'));

-- Assign existing cards to default board
UPDATE cards SET board_id = 'default' WHERE board_id IS NULL;

-- Make board_id NOT NULL after migration
-- SQLite doesn't support ALTER COLUMN, so we'll handle this in code

-- Create card_files table for file attachments
CREATE TABLE card_files (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    uploaded_at TEXT NOT NULL
);

CREATE INDEX idx_card_files_card_id ON card_files(card_id);
CREATE INDEX idx_cards_board_id ON cards(board_id);
