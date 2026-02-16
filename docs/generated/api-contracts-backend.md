# API Contracts — Backend

Base URL: `http://localhost:21547`

All request/response bodies are JSON unless noted. Errors return `{"error": "message"}`.

## Authentication

All protected routes require `Authorization: Bearer <jwt_token>` header.
MCP/service routes accept `X-Service-Key: <key>` header (localhost only).

### Auth Endpoints (Public)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/auth/register` | Register user | `{username, nickname, password, email?, first_name?, last_name?}` |
| POST | `/api/auth/login` | Login | `{username, password}` |
| POST | `/api/auth/refresh` | Refresh JWT | `{refresh_token}` |
| GET | `/api/auth/me` | Get current user | - |

### Health (Public)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |

## Board Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/boards` | List all boards | - |
| POST | `/api/boards` | Create board | `{name}` |
| PATCH | `/api/boards/{id}` | Update board | `{name}` |
| DELETE | `/api/boards/{id}` | Delete board | - |
| PATCH | `/api/boards/{id}/reorder` | Reorder board | `{position}` |

## Board Settings Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/boards/{id}/settings` | Get board settings | - |
| PUT | `/api/boards/{id}/settings` | Update board settings | `{codebase_path?, github_repo?, context_markdown?, ...}` |
| POST | `/api/boards/{id}/settings/auto-detect` | Start AI auto-detect | `{codebase_path}` |
| POST | `/api/boards/{id}/settings/clone-repo` | Clone GitHub repo | `{github_url, clone_path, pat?}` |
| GET | `/api/boards/{id}/settings/auto-detect-status` | Get auto-detect status | - |
| GET | `/api/boards/{id}/settings/auto-detect-logs` | Get auto-detect logs | - |

## Board View (Protected)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/board?board_id={id}` | Cards grouped by stage |

**Response:** `{backlog: [Card], plan: [Card], todo: [Card], in_progress: [Card], review: [Card], done: [Card]}`

Each Card in board view includes summary fields: `subtask_count`, `subtask_completed`, `label_count`, `comment_count`.

## Card Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards` | Create card | `{title, description?, stage?, priority?, board_id?, working_directory?}` |
| GET | `/api/cards/{id}` | Get card (with subtasks, labels, comments) | - |
| PATCH | `/api/cards/{id}` | Update card | `{title?, description?, priority?, working_directory?, linked_documents?, ai_agent?}` |
| DELETE | `/api/cards/{id}` | Delete card | - |
| PATCH | `/api/cards/{id}/move` | Move card to stage | `{stage, position}` |
| POST | `/api/cards/{id}/generate-plan` | Trigger AI plan generation | - |
| POST | `/api/cards/{id}/stop-ai` | Cancel active AI session | - |
| POST | `/api/cards/{id}/resume-ai` | Resume AI processing | - |
| GET | `/api/cards/{id}/logs` | Get agent logs | - |
| GET | `/api/cards/{id}/versions` | Get version history | - |
| POST | `/api/cards/{id}/versions/{vid}/restore` | Restore to version | - |
| GET | `/api/cards/{id}/diff` | Get git diff | - |
| POST | `/api/cards/{id}/merge` | Merge branch to main | - |
| POST | `/api/cards/{id}/create-pr` | Create GitHub PR | `{title?, body?}` |
| POST | `/api/cards/{id}/reject` | Reject card (back to todo) | `{feedback?}` |

### Card Object

```json
{
  "id": "uuid",
  "title": "string",
  "description": "string",
  "stage": "backlog|plan|todo|in_progress|review|done",
  "position": 1000,
  "priority": "low|medium|high|critical",
  "working_directory": "/path/to/project",
  "plan_path": "/path/to/plan.md",
  "ai_session_id": "ses_...",
  "ai_status": "idle|queued|dispatched|working|completed|failed|cancelled",
  "ai_progress": "{\"completed_todos\": 3, \"total_todos\": 10}",
  "linked_documents": "[\"/path/to/doc.md\"]",
  "ai_agent": "sisyphus",
  "branch_name": "ai/abc123-feature-name",
  "worktree_path": ".lightup-workspaces/abc123",
  "created_at": "2026-02-15T08:00:00Z",
  "updated_at": "2026-02-15T08:00:00Z"
}
```

### Stage Transition Rules

| From | Allowed To |
|------|-----------|
| Backlog | Plan, Backlog |
| Plan | Todo, Backlog |
| Todo | In Progress, Backlog |
| In Progress | Review, Backlog |
| Review | Done, Todo, Backlog |
| Done | Backlog |

## Subtask Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards/{id}/subtasks` | Create subtask | `{title, phase?, phase_order?}` |
| PATCH | `/api/subtasks/{id}` | Update subtask | `{title?, completed?, position?, phase?, phase_order?}` |
| DELETE | `/api/subtasks/{id}` | Delete subtask | - |

## Comment Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/cards/{id}/comments` | List comments | - |
| POST | `/api/cards/{id}/comments` | Create comment | `{author, content}` |
| PATCH | `/api/comments/{id}` | Update comment | `{content}` |
| DELETE | `/api/comments/{id}` | Delete comment | - |

## Label Endpoints (Protected)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/labels` | List all labels |
| POST | `/api/cards/{id}/labels/{label_id}` | Add label to card |
| DELETE | `/api/cards/{id}/labels/{label_id}` | Remove label |

**Default labels:** Bug (red), Feature (green), Improvement (blue), Documentation (orange), Urgent (pink)

## File Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards/{id}/files` | Upload files | multipart/form-data |
| GET | `/api/cards/{id}/files` | List card files | - |
| GET | `/api/files/{id}` | Download file | - |
| DELETE | `/api/files/{id}` | Delete file | - |

## AI Question Endpoints

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/cards/{id}/questions` | Get questions (Public) | - |
| POST | `/api/cards/{id}/questions` | Create question (Public) | `{question, question_type?, options?, multiple?}` |
| POST | `/api/cards/{id}/questions/{qid}/answer` | Answer question (Protected) | `{answer}` |

## Settings Endpoints (Protected)

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/settings/{key}` | Get setting | - |
| PUT | `/api/settings/{key}` | Set setting | `{value}` |

## Picker Endpoints (Protected)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/pick-directory` | Open native directory picker |
| POST | `/api/pick-files` | Open native file picker |

## Real-Time Endpoints

### WebSocket Events

**`GET /ws/events?token=<jwt>`** — Subscribe to all board events.

20+ event types: `cardCreated`, `cardUpdated`, `cardMoved`, `cardDeleted`, `subtaskCreated`, `subtaskUpdated`, `subtaskDeleted`, `subtaskToggled`, `commentCreated`, `commentUpdated`, `commentDeleted`, `boardCreated`, `boardUpdated`, `boardDeleted`, `labelAdded`, `labelRemoved`, `aiStatusChanged`, `agentLogCreated`, `questionCreated`, `questionAnswered`, `autoDetectStatus`

### WebSocket Logs

**`GET /ws/logs/{card_id}`** — Per-card AI agent log stream. Replays existing logs on connection, then streams new ones.

### MCP Endpoint

**`POST /mcp`** — Streamable HTTP MCP endpoint. Same tools as the stdio binary.

## Total Endpoint Count

- **Auth:** 4 endpoints
- **Health:** 2 endpoints
- **Boards:** 5 endpoints
- **Board Settings:** 6 endpoints
- **Board View:** 1 endpoint
- **Cards:** 15 endpoints
- **Subtasks:** 3 endpoints
- **Comments:** 4 endpoints
- **Labels:** 3 endpoints
- **Files:** 4 endpoints
- **Questions:** 3 endpoints
- **Settings:** 2 endpoints
- **Picker:** 2 endpoints
- **Real-time:** 3 endpoints (2 WebSocket + 1 MCP)

**Total: ~57 endpoints**
