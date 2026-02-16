ALTER TABLE board_settings ADD COLUMN auto_detect_status TEXT NOT NULL DEFAULT '';
ALTER TABLE board_settings ADD COLUMN auto_detect_session_id TEXT NOT NULL DEFAULT '';
ALTER TABLE board_settings ADD COLUMN auto_detect_started_at TEXT NOT NULL DEFAULT '';
