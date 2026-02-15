CREATE TABLE IF NOT EXISTS card_versions (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    changed_by TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL,
    FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
);

CREATE INDEX idx_card_versions_card_id ON card_versions(card_id);
