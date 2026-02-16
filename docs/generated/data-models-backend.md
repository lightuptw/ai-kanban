# Data Models — Backend

## Database

**Engine:** SQLite with WAL (Write-Ahead Logging) mode
**ORM:** sqlx 0.8 (compile-time checked queries)
**File:** `backend/kanban.db` (auto-created on first startup)
**Migrations:** 17 sequential SQL files in `backend/migrations/`

## Entity-Relationship Overview

```
boards 1──* cards 1──* subtasks
                  1──* comments
                  1──* card_labels *──1 labels
                  1──* card_files
                  1──* agent_logs
                  1──* card_versions
                  1──* ai_questions
boards 1──1 board_settings
users  1──* refresh_tokens
```

## Tables

### cards

Primary work items flowing through the kanban pipeline.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| title | TEXT | NOT NULL | Card title |
| description | TEXT | '' | Rich text description |
| stage | TEXT | 'backlog' | Current pipeline stage |
| position | INTEGER | 1000 | Sort order within stage |
| priority | TEXT | 'medium' | low / medium / high / critical |
| board_id | TEXT FK | NOT NULL | Parent board reference |
| working_directory | TEXT | NULL | Project directory for AI work |
| plan_path | TEXT | NULL | Path to generated work plan |
| ai_session_id | TEXT | NULL | OpenCode session ID |
| ai_status | TEXT | 'idle' | idle/queued/dispatched/working/completed/failed/cancelled |
| ai_progress | TEXT | NULL | JSON: {completed_todos, total_todos, current_task} |
| linked_documents | TEXT | NULL | JSON array of document paths |
| ai_agent | TEXT | NULL | Agent persona (e.g., "sisyphus") |
| branch_name | TEXT | NULL | Git branch name (ai/{id}-{slug}) |
| worktree_path | TEXT | NULL | Git worktree directory |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| updated_at | TEXT | ISO 8601 | Last update timestamp |

### subtasks

Checklist items within cards, organized by phases.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Parent card reference |
| title | TEXT | NOT NULL | Subtask description |
| completed | BOOLEAN | false | Completion status |
| position | INTEGER | 0 | Sort order within phase |
| phase | TEXT | NULL | Phase grouping name |
| phase_order | INTEGER | NULL | Phase sort order |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| updated_at | TEXT | ISO 8601 | Last update timestamp |

### boards

Multiple kanban boards with ordering.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| name | TEXT | NOT NULL | Board display name |
| position | INTEGER | 0 | Sort order in sidebar |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| updated_at | TEXT | ISO 8601 | Last update timestamp |

### board_settings

Per-board configuration for AI agent context.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| board_id | TEXT PK/FK | NOT NULL | Parent board reference |
| codebase_path | TEXT | NULL | Path to project codebase |
| github_repo | TEXT | NULL | GitHub repository URL |
| context_markdown | TEXT | NULL | Free-form AI context notes |
| document_links | TEXT | NULL | Reference document paths |
| variables | TEXT | NULL | Environment variables |
| tech_stack | TEXT | NULL | Technology stack description |
| communication_patterns | TEXT | NULL | API/messaging patterns |
| environments | TEXT | NULL | Environment descriptions |
| code_conventions | TEXT | NULL | Coding standards |
| testing_requirements | TEXT | NULL | Test expectations |
| api_conventions | TEXT | NULL | API design conventions |
| infrastructure | TEXT | NULL | Infrastructure details |
| ai_concurrency | INTEGER | 1 | Max parallel AI cards |
| auto_detect_status | TEXT | NULL | idle/running/completed/failed |
| auto_detect_session_id | TEXT | NULL | OpenCode session for auto-detect |
| auto_detect_started_at | TEXT | NULL | Auto-detect start timestamp |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| updated_at | TEXT | ISO 8601 | Last update timestamp |

### labels

Color-coded tags for card categorization.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| name | TEXT | NOT NULL | Label name |
| color | TEXT | NOT NULL | Hex color code |

**Seeded defaults:** Bug (#f44336), Feature (#4caf50), Improvement (#2196f3), Documentation (#ff9800), Urgent (#e91e63)

### card_labels

Junction table for many-to-many card-label relationship.

| Column | Type | Description |
|--------|------|-------------|
| card_id | TEXT FK | Card reference |
| label_id | TEXT FK | Label reference |
| PRIMARY KEY | (card_id, label_id) | Composite key |

### comments

Discussion threads on cards.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Parent card reference |
| author | TEXT | NOT NULL | Comment author name |
| content | TEXT | NOT NULL | Comment text |
| created_at | TEXT | ISO 8601 | Creation timestamp |

### card_files

File attachments stored per card.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Parent card reference |
| filename | TEXT | NOT NULL | Storage filename |
| original_filename | TEXT | NOT NULL | Original upload name |
| file_path | TEXT | NOT NULL | Disk storage path |
| file_size | INTEGER | NOT NULL | File size in bytes |
| mime_type | TEXT | NOT NULL | MIME content type |
| uploaded_at | TEXT | ISO 8601 | Upload timestamp |

### agent_logs

Persisted AI activity events for playback.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Associated card |
| session_id | TEXT | NULL | OpenCode session ID |
| event_type | TEXT | NOT NULL | Event type (message.part.delta, session.status, etc.) |
| agent | TEXT | NULL | Agent name (build, oracle, explore, etc.) |
| content | TEXT | NULL | Event content/text |
| metadata | TEXT | NULL | Additional JSON metadata |
| created_at | TEXT | ISO 8601 | Event timestamp |

### card_versions

Snapshot history for card rollback.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Parent card reference |
| snapshot | TEXT | NOT NULL | Full card JSON snapshot |
| changed_by | TEXT | NOT NULL | Change author ("api", "ai", etc.) |
| created_at | TEXT | ISO 8601 | Snapshot timestamp |

**Retention:** Maximum 50 versions per card. Auto-snapshot taken before every card update.

### users

User accounts with secure password storage.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| tenant_id | TEXT | NOT NULL | Tenant isolation ID |
| username | TEXT | NOT NULL, UNIQUE | Login username |
| nickname | TEXT | NULL | Display name |
| first_name | TEXT | NULL | First name |
| last_name | TEXT | NULL | Last name |
| email | TEXT | NULL | Email address |
| avatar | TEXT | NULL | Avatar URL |
| password_hash | TEXT | NOT NULL | Argon2id hash |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| updated_at | TEXT | ISO 8601 | Last update timestamp |

**Seeded default:** Username `LightUp`, password `Spark123`

### refresh_tokens

JWT refresh token management.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| user_id | TEXT FK | NOT NULL | User reference |
| token_hash | TEXT | NOT NULL | SHA-256 hash of token |
| expires_at | TEXT | NOT NULL | Expiration timestamp |
| created_at | TEXT | ISO 8601 | Creation timestamp |
| revoked | BOOLEAN | false | Revocation flag |

### app_secrets

Application-level secrets (JWT signing key, service API key).

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| key | TEXT PK | NOT NULL | Secret identifier |
| value | TEXT | NOT NULL | Secret value |
| created_at | TEXT | ISO 8601 | Creation timestamp |

**Known keys:** `jwt_signing_key`, `service_api_key`

### ai_questions

AI-to-user questions with answers.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| id | TEXT PK | UUID v4 | Unique identifier |
| card_id | TEXT FK | NOT NULL | Associated card |
| session_id | TEXT | NULL | OpenCode session ID |
| question | TEXT | NOT NULL | Question text |
| question_type | TEXT | 'text' | text / select / multi_select |
| options | TEXT | NULL | JSON array of {label, description} |
| multiple | BOOLEAN | false | Allow multiple selections |
| answer | TEXT | NULL | User's answer |
| answered_at | TEXT | NULL | Answer timestamp |
| created_at | TEXT | ISO 8601 | Creation timestamp |

### app_settings

Key-value application settings.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| key | TEXT PK | NOT NULL | Setting name |
| value | TEXT | NOT NULL | Setting value |

## Rust Domain Types

```rust
// Core domain structs (backend/src/domain/card.rs)
pub struct Card { id, title, description, stage, position, priority, board_id, working_directory, plan_path, ai_session_id, ai_status, ai_progress, linked_documents, ai_agent, branch_name, worktree_path, created_at, updated_at }
pub struct Subtask { id, card_id, title, completed, position, phase, phase_order, created_at, updated_at }
pub struct Label { id, name, color }
pub struct Comment { id, card_id, author, content, created_at }
pub struct AgentLog { id, card_id, session_id, event_type, agent, content, metadata, created_at }
pub struct CardVersion { id, card_id, snapshot, changed_by, created_at }
pub struct AiQuestion { id, card_id, session_id, question, question_type, options, multiple, answer, answered_at, created_at }

// Stage enum with transition validation (backend/src/domain/stage.rs)
pub enum Stage { Backlog, Plan, Todo, InProgress, Review, Done }

// Error types (backend/src/domain/error.rs)
pub enum KanbanError { NotFound, BadRequest(String), Database(String), OpenCodeError(String), Internal(String) }
```

## Migration History

| # | File | Date | Changes |
|---|------|------|---------|
| 1 | `20260214_001_initial.sql` | Feb 14 | cards, subtasks, labels, card_labels, comments + WAL mode + seed labels |
| 2 | `20260215_001_boards_and_files.sql` | Feb 15 | boards, card_files, cards.board_id |
| 3 | `20260216_001_subtask_phases.sql` | Feb 16 | subtasks.phase, subtasks.phase_order |
| 4 | `20260217_001_board_position.sql` | Feb 17 | boards.position |
| 5 | `20260218_001_fix_null_board_ids.sql` | Feb 18 | Backfill null board_ids |
| 6 | `20260219_001_settings.sql` | Feb 19 | app_settings table |
| 7 | `20260220_001_agent_logs.sql` | Feb 20 | agent_logs table |
| 8 | `20260221_001_add_ai_agent.sql` | Feb 21 | cards.ai_agent |
| 9 | `20260222_001_add_card_versions.sql` | Feb 22 | card_versions table |
| 10 | `20260223_001_board_settings.sql` | Feb 23 | board_settings table |
| 11 | `20260224_001_board_settings_github_repo.sql` | Feb 24 | board_settings.github_repo |
| 12 | `20260225_001_auth.sql` | Feb 25 | app_secrets, users tables |
| 13 | `20260226_001_refresh_tokens.sql` | Feb 26 | refresh_tokens table |
| 14 | `20260227_001_ai_questions.sql` | Feb 27 | ai_questions table |
| 15 | `20260228_001_board_settings_concurrency.sql` | Feb 28 | board_settings.ai_concurrency |
| 16 | `20260301_001_card_worktree.sql` | Mar 1 | cards.branch_name, cards.worktree_path |
| 17 | `20260302_001_auto_detect_status.sql` | Mar 2 | board_settings auto_detect fields |
