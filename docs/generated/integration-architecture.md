# Integration Architecture

## System Overview

The LightUp AI Kanban system consists of 4 runtime components communicating via HTTP, WebSocket, SSE, and stdio:

```
┌─────────────────┐    REST + WS     ┌────────────────────────┐
│   Frontend      │ ──────────────> │   Backend (Rust/Axum)   │
│   React SPA     │ <────────────── │   Port :21547           │
│   Port :21548   │    WebSocket    │                         │
│                 │    /ws/events   │   - REST API (JWT)      │     SQLite
│  - KanbanBoard  │    /ws/logs/*   │   - WebSocket server    │ ──────> kanban.db
│  - CardDialog   │                 │   - Queue processor     │
│  - DiffViewer   │                 │   - SSE relay           │
│  - BoardSettings│                 │   - Static file serving │
│  - Redux store  │                 │   - MCP endpoint (/mcp) │
└─────────────────┘                 └───────────┬────────────┘
                                                │
                                    HTTP: POST /session
                                    HTTP: POST /session/{id}/message
                                    HTTP: POST /session/{id}/abort
                                    SSE:  GET /event
                                                │
                                                v
                                    ┌────────────────────────┐
                                    │   OpenCode CLI         │
                                    │   Port :4096           │
                                    │                        │
                                    │   - AI sessions        │
                                    │   - SSE event stream   │
                                    │   - MCP tool clients   │
                                    └───────────┬────────────┘
                                                │ stdio
                                                v
                                    ┌────────────────────────┐
                                    │   kanban-mcp           │    HTTP
                                    │   (stdio binary)       │ ──────> Backend :21547
                                    │   20+ MCP tools        │
                                    └────────────────────────┘
```

## Integration Points

### 1. Frontend → Backend (REST API)

| From | To | Protocol | Description |
|------|----|----------|-------------|
| Frontend (api.ts) | Backend (:21547) | HTTP REST | All CRUD operations |
| Frontend (auth.ts) | Backend (:21547) | HTTP REST | Authentication (JWT) |

- **Auth:** Bearer JWT token in `Authorization` header
- **Base URL:** Dynamic `{protocol}//{hostname}:21547` or `VITE_API_URL` env
- **Content-Type:** `application/json` (except file uploads: `multipart/form-data`)
- **Error handling:** Auto-refresh JWT on 401, redirect to /login on refresh failure

### 2. Frontend ↔ Backend (WebSocket)

| From | To | Protocol | Path | Description |
|------|----|----------|------|-------------|
| Frontend (sse.ts) | Backend | WebSocket | `/ws/events?token=<jwt>` | All real-time events (20+ types) |
| Frontend (AgentLogViewer) | Backend | WebSocket | `/ws/logs/{card_id}` | Per-card AI agent logs |

- **Events WebSocket:** Single connection, multiplexed events for all boards/cards
- **Logs WebSocket:** One connection per open card detail dialog
- **Reconnection:** Exponential backoff (1s → 30s max)
- **Auth:** JWT token passed as query parameter

### 3. Backend → OpenCode (HTTP + SSE)

| From | To | Protocol | Path | Description |
|------|----|----------|------|-------------|
| AiDispatchService | OpenCode (:4096) | HTTP POST | `/session` | Create AI session |
| AiDispatchService | OpenCode (:4096) | HTTP POST | `/session/{id}/message` | Send work prompt |
| Cards handler | OpenCode (:4096) | HTTP POST | `/session/{id}/abort` | Stop AI session |
| SseRelayService | OpenCode (:4096) | SSE GET | `/event` | Subscribe to all AI events |

- **Base URL:** `OPENCODE_URL` env var (default: `http://localhost:4096`)
- **Connection:** SseRelayService maintains persistent SSE connection with auto-reconnect
- **Event filtering:** Backend filters noise events before broadcasting to clients

### 4. OpenCode → kanban-mcp (stdio)

| From | To | Protocol | Description |
|------|----|----------|-------------|
| OpenCode | kanban-mcp binary | stdio (JSON-RPC) | MCP tool calls |

- **Transport:** Standard input/output (JSON-RPC 2.0)
- **Configuration:** Defined in `~/.config/opencode/opencode.json`
- **Binary:** `backend/target/release/kanban-mcp`

### 5. kanban-mcp → Backend (HTTP)

| From | To | Protocol | Description |
|------|----|----------|-------------|
| kanban-mcp | Backend (:21547) | HTTP REST | All MCP tool calls proxied |

- **Base URL:** `KANBAN_API_URL` env var (default: `http://127.0.0.1:21547`)
- **Auth:** `X-Service-Key` header (from `.service-key` file or `KANBAN_SERVICE_KEY` env)
- **Design:** Stateless HTTP proxy — no direct database access

## Data Flow: Card Lifecycle

### 1. Card Creation
```
User → Frontend (KanbanBoard) → POST /api/cards → Backend → SQLite
                                                    │
                                                    └─ WebSocket broadcast: cardCreated
                                                    │
                                        Frontend (all clients) ← updateCardFromSSE
```

### 2. AI Plan Generation
```
User clicks "Generate Plan"
  → Frontend → POST /api/cards/{id}/generate-plan → Backend
  → Backend → POST /session → OpenCode (creates AI session)
  → Backend stores session_id, sets ai_status=planning
  → Backend → POST /session/{id}/message → OpenCode (sends prompt)
  → OpenCode uses kanban-mcp tools (stdio) → kanban-mcp → Backend REST API
  → WebSocket broadcast: aiStatusChanged → Frontend updates UI
```

### 3. AI Auto-Dispatch (Queue)
```
User moves card to Todo → ai_status=queued
  → QueueProcessor (every 3s) picks queued card
  → GitWorktreeService creates branch + worktree
  → PlanGenerator writes .sisyphus/plans/{slug}.md
  → AiDispatchService → POST /session → OpenCode
  → AiDispatchService → POST /session/{id}/message → OpenCode
  → SseRelayService ← SSE /event ← OpenCode
  → SseRelayService persists to agent_logs, updates ai_status
  → WebSocket broadcast: aiStatusChanged, agentLogCreated
  → Frontend updates KanbanCard Larson scanner, AgentLogViewer
```

### 4. AI Completion & Review
```
OpenCode AI finishes work
  → SSE event: session.idle → SseRelayService
  → SseRelayService sets ai_status=completed, moves card to review
  → WebSocket broadcast: aiStatusChanged, cardMoved
  → Frontend: card appears in Review column
  → User opens card → loads diff (GET /api/cards/{id}/diff)
  → DiffViewer shows file changes
  → User clicks Merge → POST /api/cards/{id}/merge → Git merge
  → User clicks Create PR → POST /api/cards/{id}/create-pr → GitHub API
  → User clicks Reject → POST /api/cards/{id}/reject → card back to Todo
```

### 5. AI Questions Mid-Task
```
AI agent needs user input
  → kanban-mcp → POST /api/cards/{id}/questions → Backend
  → Backend stores AiQuestion, broadcasts questionCreated
  → Frontend: KanbanCard shows waiting_input indicator
  → User opens card, sees question with options
  → User answers → POST /api/cards/{id}/questions/{qid}/answer
  → Backend broadcasts questionAnswered
  → AI agent resumes work
```

## Shared Data Contracts

### Card (shared between all components)

Both frontend TypeScript type and backend Rust struct share the same field set:
- `id`, `title`, `description`, `stage`, `position`, `priority`
- `board_id`, `working_directory`, `plan_path`
- `ai_session_id`, `ai_status`, `ai_progress`
- `linked_documents`, `ai_agent`, `branch_name`, `worktree_path`
- `created_at`, `updated_at`

### Stage Values

Consistent across all components: `backlog`, `plan`, `todo`, `in_progress`, `review`, `done`

### AI Status Values

Consistent across all components: `idle`, `queued`, `dispatched`, `working`, `completed`, `failed`, `cancelled`

## Environment Configuration

| Variable | Component | Default | Description |
|----------|-----------|---------|-------------|
| `PORT` | Backend | 21547 | HTTP server port |
| `DATABASE_URL` | Backend | sqlite:kanban.db | SQLite path |
| `OPENCODE_URL` | Backend | http://localhost:4096 | OpenCode API |
| `FRONTEND_DIR` | Backend | ../frontend/dist | Static files |
| `CORS_ORIGIN` | Backend | http://localhost:21548 | Allowed origins |
| `KANBAN_API_URL` | kanban-mcp | http://127.0.0.1:21547 | Backend API URL |
| `KANBAN_SERVICE_KEY` | kanban-mcp | (from .service-key) | Auth key |
| `VITE_API_URL` | Frontend | (dynamic) | API base URL |
