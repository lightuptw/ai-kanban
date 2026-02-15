CREATE TABLE IF NOT EXISTS ai_questions (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    question TEXT NOT NULL,
    question_type TEXT NOT NULL DEFAULT 'select',
    options TEXT NOT NULL DEFAULT '[]',
    multiple INTEGER NOT NULL DEFAULT 0,
    answer TEXT,
    answered_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_questions_card ON ai_questions(card_id);
CREATE INDEX IF NOT EXISTS idx_ai_questions_session ON ai_questions(session_id);
