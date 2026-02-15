-- Board-level settings: AI context, codebase path, documents, variables
CREATE TABLE IF NOT EXISTS board_settings (
    board_id TEXT PRIMARY KEY REFERENCES boards(id) ON DELETE CASCADE,
    codebase_path TEXT NOT NULL DEFAULT '',
    context_markdown TEXT NOT NULL DEFAULT '',
    document_links TEXT NOT NULL DEFAULT '[]',
    variables TEXT NOT NULL DEFAULT '{}',
    tech_stack TEXT NOT NULL DEFAULT '',
    communication_patterns TEXT NOT NULL DEFAULT '',
    environments TEXT NOT NULL DEFAULT '',
    code_conventions TEXT NOT NULL DEFAULT '',
    testing_requirements TEXT NOT NULL DEFAULT '',
    api_conventions TEXT NOT NULL DEFAULT '',
    infrastructure TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
