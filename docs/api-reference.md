# API Reference

Base URL: `http://localhost:3000`

All request/response bodies are JSON. Errors return `{"error": "message", "status": <code>}`.

## Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/health/live` | Liveness probe |

## Board View

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/board?board_id={id}` | Cards grouped by stage |

**Response:**

```json
{
  "backlog": [Card, ...],
  "plan": [Card, ...],
  "todo": [Card, ...],
  "in_progress": [Card, ...],
  "review": [Card, ...],
  "done": [Card, ...]
}
```

Each Card in the board view includes `subtask_count`, `subtask_completed`, `label_count`, `comment_count` summary fields.

## Boards

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/boards` | List all boards | - |
| POST | `/api/boards` | Create board | `{name}` |
| PATCH | `/api/boards/{id}` | Update board | `{name}` |
| DELETE | `/api/boards/{id}` | Delete board | - |
| PATCH | `/api/boards/{id}/reorder` | Reorder board | `{position}` |

**Board object:** `{id, name, position, created_at, updated_at}`

## Cards

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards` | Create card | `{title, description?, stage?, priority?, board_id?, working_directory?}` |
| GET | `/api/cards/{id}` | Get card (with subtasks, labels, comments) | - |
| PATCH | `/api/cards/{id}` | Update card | `{title?, description?, priority?, working_directory?, linked_documents?, ai_agent?}` |
| DELETE | `/api/cards/{id}` | Delete card | - |
| PATCH | `/api/cards/{id}/move` | Move card to stage | `{stage, position}` |
| POST | `/api/cards/{id}/generate-plan` | Trigger AI plan generation | - |
| POST | `/api/cards/{id}/stop-ai` | Cancel active AI session | - |
| GET | `/api/cards/{id}/logs` | Get agent logs | - |
| GET | `/api/cards/{id}/versions` | Get version history | - |
| POST | `/api/cards/{id}/versions/{vid}/restore` | Restore to version | - |

**Card object:**

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
  "ai_agent": "bmad-master",
  "created_at": "2026-02-15T08:00:00Z",
  "updated_at": "2026-02-15T08:00:00Z"
}
```

**Stage values:** `backlog`, `plan`, `todo`, `in_progress`, `review`, `done`

**Move card** — enforces transition rules (see [Architecture](architecture.md#stage-transition-rules)).

**Generate plan** — requires card to be in `plan` stage. Returns updated card. AI works asynchronously; progress tracked via SSE/WebSocket.

**Stop AI** — requires card to have an active AI session (`ai_status` in: planning, dispatched, working, queued). Calls opencode abort API, marks card as `cancelled`.

## Subtasks

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards/{id}/subtasks` | Create subtask | `{title, phase?, phase_order?}` |
| PATCH | `/api/subtasks/{id}` | Update subtask | `{title?, completed?, position?, phase?, phase_order?}` |
| DELETE | `/api/subtasks/{id}` | Delete subtask | - |

**Subtask object:** `{id, card_id, title, completed, position, phase, phase_order, created_at, updated_at}`

Subtasks are grouped by `phase` (string) and ordered by `phase_order` (phase sorting) then `position` (within phase).

## Comments

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/cards/{id}/comments` | List comments | - |
| POST | `/api/cards/{id}/comments` | Create comment | `{author, content}` |
| PATCH | `/api/comments/{id}` | Update comment | `{content}` |
| DELETE | `/api/comments/{id}` | Delete comment | - |

**Comment object:** `{id, card_id, author, content, created_at}`

## Labels

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/labels` | List all labels | - |
| POST | `/api/cards/{id}/labels/{label_id}` | Add label to card | - |
| DELETE | `/api/cards/{id}/labels/{label_id}` | Remove label from card | - |

**Default labels:** Bug (red), Feature (green), Improvement (blue), Documentation (orange), Urgent (pink)

## Files

| Method | Path | Description | Body |
|--------|------|-------------|------|
| POST | `/api/cards/{id}/files` | Upload files | multipart/form-data |
| GET | `/api/cards/{id}/files` | List card files | - |
| GET | `/api/files/{id}` | Download file | - |
| DELETE | `/api/files/{id}` | Delete file | - |

## Settings

| Method | Path | Description | Body |
|--------|------|-------------|------|
| GET | `/api/settings/{key}` | Get setting | - |
| PUT | `/api/settings/{key}` | Set setting | `{value}` |

**Response:** `{key, value, updated_at}`

Known keys: `ai_concurrency` (default: "1")

## Picker (Native OS Dialogs)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/pick-directory` | Open native directory picker dialog |
| POST | `/api/pick-files` | Open native file picker dialog |

Returns selected path(s). Only works when backend runs on a desktop OS.

## Real-time Endpoints

### SSE Events

**`GET /api/events`** — Server-Sent Events stream.

Event types:

| Event | Payload | When |
|-------|---------|------|
| `BoardUpdated` | `{board_id}` | Card created/updated/deleted |
| `CardUpdated` | `{card_id}` | Card fields changed |
| `AiStatusChanged` | `{card_id, status, progress, stage, ai_session_id}` | AI status transition |

### WebSocket Logs

**`GET /ws/logs/{card_id}`** — WebSocket connection for live agent logs.

Sends JSON messages with AgentLog objects as AI processes the card. Replays existing logs on connection, then streams new ones in real-time.

### MCP Endpoint

**`POST /mcp`** — Streamable HTTP MCP endpoint.

Alternative to the stdio binary. Same 15 tools, same HTTP proxy behavior. Used for direct MCP tool calls without opencode.

## Agent Log Object

```json
{
  "id": "uuid",
  "card_id": "uuid",
  "session_id": "ses_...",
  "event_type": "message.part.delta|session.status|todo.updated|...",
  "agent": "sisyphus",
  "content": "AI output text",
  "metadata": "{}",
  "created_at": "2026-02-15T08:00:00Z"
}
```

## Card Version Object

```json
{
  "id": "uuid",
  "card_id": "uuid",
  "snapshot": "{...card JSON...}",
  "changed_by": "api",
  "created_at": "2026-02-15T08:00:00Z"
}
```

Maximum 50 versions retained per card. Auto-snapshot taken before every card update.
