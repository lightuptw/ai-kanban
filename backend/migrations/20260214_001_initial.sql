-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;

-- Cards table
CREATE TABLE IF NOT EXISTS cards (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    stage TEXT NOT NULL DEFAULT 'backlog',
    position INTEGER NOT NULL DEFAULT 0,
    priority TEXT NOT NULL DEFAULT 'medium',
    working_directory TEXT NOT NULL DEFAULT '.',
    plan_path TEXT,
    ai_session_id TEXT,
    ai_status TEXT NOT NULL DEFAULT 'idle',
    ai_progress TEXT NOT NULL DEFAULT '{}',
    linked_documents TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Subtasks table
CREATE TABLE IF NOT EXISTS subtasks (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Labels table
CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL
);

-- Card-Label junction table
CREATE TABLE IF NOT EXISTS card_labels (
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, label_id)
);

-- Comments table
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    author TEXT NOT NULL DEFAULT 'user',
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Seed default labels
INSERT OR IGNORE INTO labels (id, name, color) VALUES
    ('lbl-bug', 'Bug', '#f44336'),
    ('lbl-feature', 'Feature', '#4caf50'),
    ('lbl-improvement', 'Improvement', '#2196f3'),
    ('lbl-docs', 'Documentation', '#ff9800'),
    ('lbl-urgent', 'Urgent', '#e91e63');
