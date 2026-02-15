# Architecture

## System Overview

The system consists of three processes:

| Process | Port | Role |
|---------|------|------|
| **Backend** (Rust/Axum) | :3000 | REST API, WebSocket, SSE events, static file serving, MCP endpoint |
| **Frontend** (Vite/React) | :5173 | SPA dev server (in production, served by backend from `frontend/dist`) |
| **opencode CLI** | :4096 | AI agent runtime with session management and MCP tool support |

## System Diagram

```
┌─────────────────┐      HTTP/REST        ┌───────────────────────┐
│    Frontend      │ ────────────────────> │   Backend (Axum)      │
│    React SPA     │ <──────────────────── │   Port :3000          │
│    Port :5173    │    SSE /api/events    │                       │
│                  │ <──── WebSocket ───── │   - REST API          │    SQLite
│  - KanbanBoard   │    /ws/logs/{id}      │   - SSE broadcast     │ ──────────> kanban.db
│  - CardDialog    │                       │   - WebSocket logs    │
│  - AgentLogView  │                       │   - Static files      │
│  - Redux store   │                       │   - MCP /mcp endpoint │
└─────────────────┘                       └───────────┬───────────┘
                                                      │
                                          HTTP: POST /session
                                          HTTP: POST /session/{id}/message
                                          HTTP: POST /session/{id}/abort
                                          SSE:  GET /event
                                                      │
                                                      v
                                          ┌───────────────────────┐
                                          │   opencode CLI        │
                                          │   Port :4096          │
                                          │                       │
                                          │   - AI sessions       │
                                          │   - SSE event stream  │
                                          │   - MCP tool clients  │
                                          └───────────┬───────────┘
                                                      │ stdio
                                                      v
                                          ┌───────────────────────┐
                                          │   kanban-mcp          │    HTTP
                                          │   (stdio binary)      │ ──────────> Backend :3000
                                          │   15 MCP tools        │
                                          └───────────────────────┘
```

## Backend Architecture

Layered structure under `backend/src/`:

### API Layer (`src/api/`)

| File | Handlers |
|------|----------|
| `handlers/boards.rs` | list_boards, create_board, update_board, delete_board, reorder_board |
| `handlers/cards.rs` | create_card, get_card, get_board, update_card, delete_card, move_card, generate_plan, stop_ai, get_card_logs, list_card_versions, restore_card_version |
| `handlers/subtasks.rs` | create_subtask, update_subtask, delete_subtask |
| `handlers/comments.rs` | get_comments, create_comment, update_comment, delete_comment |
| `handlers/labels.rs` | list_labels, add_label, remove_label |
| `handlers/files.rs` | upload_files, list_card_files, download_file, delete_file |
| `handlers/settings.rs` | get_setting, set_setting |
| `handlers/picker.rs` | pick_directory, pick_files (native OS dialogs) |
| `handlers/sse.rs` | sse_handler (SSE event stream + SseEvent enum) |
| `handlers/ws.rs` | ws_logs_handler (WebSocket for per-card agent logs) |
| `routes.rs` | Route definitions, CORS config, static file serving |
| `state.rs` | AppState: db pool, SSE broadcast channel, HTTP client, config |
| `dto/` | Request/response types (CreateCardRequest, CardResponse, BoardResponse, etc.) |

### Service Layer (`src/services/`)

| Service | Purpose | Key Methods |
|---------|---------|-------------|
| `CardService` | Card CRUD, board queries, version snapshots | get_card_by_id, create_card, update_card, move_card, delete_card, snapshot_version |
| `AiDispatchService` | OpenCode session management | dispatch_card (creates session + sends prompt), abort_session |
| `QueueProcessor` | Todo queue with concurrency control | start (polls every 5s), picks queued cards, dispatches, handles stuck recovery |
| `SseRelayService` | OpenCode SSE event bridge | start (connects to opencode SSE), filters noise, persists logs, broadcasts via WebSocket |
| `PlanGenerator` | Work plan file generation | generate_plan (markdown), write_plan_file (to .sisyphus/plans/) |

### Domain Layer (`src/domain/`)

| Type | Fields |
|------|--------|
| `Card` | id, title, description, stage, position, priority, working_directory, plan_path, ai_session_id, ai_status, ai_progress, linked_documents, ai_agent, created_at, updated_at |
| `Subtask` | id, card_id, title, completed, position, phase, phase_order, created_at, updated_at |
| `Label` | id, name, color |
| `Comment` | id, card_id, author, content, created_at |
| `AgentLog` | id, card_id, session_id, event_type, agent, content, metadata, created_at |
| `CardVersion` | id, card_id, snapshot (JSON), changed_by, created_at |
| `Stage` (enum) | Backlog, Plan, Todo, InProgress, Review, Done |
| `KanbanError` (enum) | NotFound, BadRequest, Database, OpenCodeError, Internal |

### MCP Layer (`src/mcp/`)

Stateless HTTP proxy. `KanbanMcp` holds a `reqwest::Client` and `base_url`. All 15 tools forward to the REST API. No direct database access.

### Binaries

| Binary | Entry Point | Purpose |
|--------|------------|---------|
| `kanban-backend` | `src/main.rs` | Main server (API + SSE relay + queue processor) |
| `kanban-mcp` | `src/bin/mcp_server.rs` | stdio MCP server for opencode |

## Card Lifecycle

```
                    User action              AI action
                    ───────────              ─────────
  ┌─────────┐
  │ Backlog │  User creates card
  └────┬────┘
       │ User moves
       v
  ┌─────────┐
  │  Plan   │  User clicks "Generate Plan"
  │         │  ──> AI creates subtasks via MCP ──>
  └────┬────┘
       │ User moves (human decision)
       v
  ┌─────────┐
  │  Todo   │  ai_status = queued
  │ (queue) │  QueueProcessor picks up card
  └────┬────┘
       │ QueueProcessor dispatches
       v
  ┌────────────┐
  │In Progress │  ai_status = working
  │            │  AI executes plan, SSE tracks progress
  └────┬───────┘
       │ AI completes (session.idle)
       v
  ┌─────────┐
  │ Review  │  ai_status = completed
  │         │  Human reviews work
  └────┬────┘
       │ User approves        │ User rejects
       v                      v
  ┌─────────┐           Back to Todo
  │  Done   │           (re-queued)
  └─────────┘
```

## Stage Transition Rules

| From | Allowed To |
|------|-----------|
| Backlog | Plan, Backlog |
| Plan | Todo, Backlog |
| Todo | In Progress, Backlog |
| In Progress | Review, Backlog |
| Review | Done, Todo, Backlog |
| Done | Backlog |

Any stage can return to Backlog.

## AI Status State Machine

```
idle ──> queued ──> dispatched ──> working ──> completed
                                     │
                                     ├──> failed
                                     │
                                     └──> cancelled (via Stop AI)
```

## SSE Event Flow

```
opencode (:4096)                   Backend (:3000)                    Frontend
─────────────────                  ───────────────                    ────────
  SSE /event ──────────────────>  SseRelayService
                                    │
                                    ├─ Filter (skip noise)
                                    ├─ Map session_id → card_id
                                    ├─ Persist to agent_logs table
                                    ├─ Update card ai_status
                                    ├─ Broadcast SSE event ─────────> SSE listener (sse.ts)
                                    └─ Broadcast WebSocket ─────────> AgentLogViewer
```

**Filtered out:** message.part.updated, session.diff, server.connected, server.heartbeat, message.updated (without finish field)

**Kept:** session.status (busy/idle), session.idle, todo.updated, message.updated (with finish), message.part.delta

## Database Schema

9 migrations in `backend/migrations/`:

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `cards` | Work items | id, title, description, stage, priority, ai_status, ai_session_id, ai_agent, linked_documents |
| `subtasks` | Card checklist items | id, card_id, title, completed, phase, phase_order, position |
| `labels` | Color-coded tags | id, name, color (5 defaults seeded) |
| `card_labels` | Card-label junction | card_id, label_id |
| `comments` | Card discussion | id, card_id, author, content |
| `boards` | Multiple boards | id, name, position |
| `card_files` | File attachments | id, card_id, filename, filepath, content_type, size |
| `agent_logs` | AI activity logs | id, card_id, session_id, event_type, agent, content, metadata |
| `settings` | Key-value config | key, value (ai_concurrency stored here) |
| `card_versions` | Version history | id, card_id, snapshot (JSON), changed_by |

All tables use TEXT primary keys (UUIDs). Timestamps stored as ISO 8601 TEXT. SQLite WAL mode enabled for concurrency.

## Startup Sequence

1. Load config from environment (or defaults)
2. Initialize SQLite database, run migrations
3. Start SSE relay (connects to opencode SSE stream)
4. Start queue processor (polls for queued cards)
5. Create Axum router with all routes + CORS + MCP endpoint
6. Bind to `0.0.0.0:{PORT}`, serve with graceful shutdown
