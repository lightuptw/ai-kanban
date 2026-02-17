CREATE TABLE IF NOT EXISTS session_mappings (
    child_session_id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    parent_session_id TEXT NOT NULL,
    agent_type TEXT,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_session_mappings_card_id ON session_mappings(card_id);
CREATE INDEX IF NOT EXISTS idx_session_mappings_parent ON session_mappings(parent_session_id);
