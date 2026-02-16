# Architecture — Backend

## Overview

The backend is a Rust application built on Axum 0.8 with Tokio async runtime. It serves REST APIs, WebSocket connections, SSE events, and an MCP endpoint. Two binaries are produced: `kanban-backend` (main server) and `kanban-mcp` (stdio MCP proxy).

## Technology Stack

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| Language | Rust | 1.75+ (edition 2021) | Systems programming |
| Async Runtime | Tokio | 1.x (full features) | Async I/O, task spawning |
| HTTP Framework | Axum | 0.8 | Routing, middleware, WebSocket |
| Database | SQLite | via sqlx 0.8 | Persistent storage (WAL mode) |
| Serialization | serde + serde_json | 1.0 | JSON serialization |
| HTTP Client | reqwest | 0.12 | OpenCode API calls |
| SSE Client | reqwest-eventsource | 0.6 | OpenCode event stream |
| MCP Protocol | rmcp | 0.15 | Model Context Protocol server |
| JWT Auth | jsonwebtoken | 9 | Token creation/verification |
| Password Hashing | argon2 + password-hash | 0.5 | Argon2id password security |
| UUID | uuid | 1.x (v4) | Unique identifier generation |
| Timestamps | chrono | 0.4 | Date/time handling |
| Error Handling | thiserror + anyhow | 2.0 / 1.0 | Error types |
| Logging | tracing + tracing-subscriber | 0.1 / 0.3 | Structured logging |
| CORS/Tracing | tower-http | 0.6 | HTTP middleware |
| Streaming | tokio-stream + futures | 0.1 / 0.3 | Async streams |
| Schema Gen | schemars | 1 | JSON Schema for MCP tools |

## Architecture Pattern

**Layered Architecture** with clear separation of concerns:

```
┌────────────────────────────────────────────────────────┐
│  API Layer (api/)                                       │
│  ├── routes.rs         Route definitions, CORS          │
│  ├── state.rs          AppState (shared server state)   │
│  ├── dto/              Request/Response types            │
│  └── handlers/         13 handler modules               │
├────────────────────────────────────────────────────────┤
│  Auth Layer (auth/)                                     │
│  ├── middleware.rs      JWT + Service Key validation     │
│  ├── handlers.rs        Auth endpoints                  │
│  ├── jwt.rs             Token management                │
│  ├── password.rs        Argon2 hashing                  │
│  └── seed.rs            Default user + service account  │
├────────────────────────────────────────────────────────┤
│  Service Layer (services/)                              │
│  ├── card_service.rs    CRUD, versions, board queries   │
│  ├── ai_dispatch.rs     OpenCode session management     │
│  ├── queue_processor.rs Background job queue            │
│  ├── sse_relay.rs       OpenCode → client event bridge  │
│  ├── git_worktree.rs    Git operations                  │
│  └── plan_generator.rs  Work plan generation            │
├────────────────────────────────────────────────────────┤
│  Domain Layer (domain/)                                 │
│  ├── card.rs            Entity structs                  │
│  ├── stage.rs           Stage enum + transitions        │
│  └── error.rs           Error types                     │
├────────────────────────────────────────────────────────┤
│  Infrastructure Layer (infrastructure/)                  │
│  └── db.rs              SQLite pool, migrations         │
├────────────────────────────────────────────────────────┤
│  MCP Layer (mcp/)                                       │
│  └── mod.rs             20+ MCP tools, HTTP proxy       │
└────────────────────────────────────────────────────────┘
```

## AppState (Shared Server State)

```rust
pub struct AppState {
    pub db: SqlitePool,           // Database connection pool
    pub tx: broadcast::Sender<WsEvent>, // WebSocket/SSE broadcast channel
    pub http_client: reqwest::Client,   // Shared HTTP client (for OpenCode)
    pub config: Config,           // Environment configuration
}
```

## Startup Sequence

1. Load `Config` from environment variables (with defaults)
2. Initialize SQLite database pool with WAL mode
3. Run all 17 migrations sequentially
4. Seed default user (`LightUp`/`Spark123`) and service account (`__kanban_ai__`)
5. Create `AppState` with broadcast channel (capacity: 1024)
6. Spawn `SseRelayService` background task (connects to OpenCode SSE)
7. Spawn `QueueProcessor` background task (polls every 3s)
8. Build Axum router with public + protected routes, CORS, static files
9. Optionally register MCP endpoint at `/mcp` (streamable HTTP)
10. Bind to `0.0.0.0:{PORT}` and serve with graceful shutdown

## Background Services

### QueueProcessor

- **Polling interval:** 3 seconds
- **Job:** Finds cards with `ai_status=queued`, dispatches to OpenCode
- **Concurrency:** Respects per-board `ai_concurrency` setting
- **Priority:** Dispatches oldest queued card first
- **Flow:** queued → create git worktree → generate plan → dispatch to OpenCode → dispatched
- **Recovery:** Cards stuck in `dispatched` for >30 minutes are recovered

### SseRelayService

- **Connection:** Subscribes to `GET {OPENCODE_URL}/event` SSE stream
- **Reconnect:** Exponential backoff (1s → 2s → 4s → ... → 30s max)
- **Event pipeline:** Filter → Map session_id to card_id → Persist to agent_logs → Update card ai_status → Broadcast via WebSocket/SSE
- **Filtered out:** `message.part.updated`, `session.diff`, `server.connected`, `server.heartbeat`
- **Kept:** `session.status`, `session.idle`, `todo.updated`, `message.updated` (with finish), `message.part.delta`

## Auth System

### JWT Flow
1. User logs in with username/password → `POST /api/auth/login`
2. Backend verifies password (Argon2id) → issues JWT (1h) + refresh token (7d)
3. Frontend stores both tokens in localStorage
4. All API calls include `Authorization: Bearer <jwt>`
5. On 401, frontend calls `POST /api/auth/refresh` with refresh token
6. JWT signing key stored in `app_secrets` table (generated on first startup)

### Service Key Flow
1. Service account `__kanban_ai__` created on startup
2. API key written to `backend/.service-key` file
3. MCP binary reads key from file or `KANBAN_SERVICE_KEY` env var
4. Sends as `X-Service-Key` header (localhost-only restriction)

## AI Status State Machine

```
idle ──> queued ──> dispatched ──> working ──> completed
                                     │
                                     ├──> failed
                                     └──> cancelled (via Stop AI)
```

## WebSocket Event System

All mutations broadcast `WsEvent` variants through a `tokio::sync::broadcast` channel. Connected WebSocket clients receive events in real-time. 20+ event types cover every entity mutation.

## MCP Server

The `KanbanMcp` struct implements the MCP protocol via `rmcp` 0.15. It operates as a **stateless HTTP proxy** — every tool call is translated into an HTTP request to the backend REST API. This ensures a single source of truth and prevents the database-path mismatch bugs that affected earlier direct-access designs.

Two transport modes:
- **stdio:** `kanban-mcp` binary for OpenCode integration
- **Streamable HTTP:** `/mcp` endpoint on the main server for direct access

## Testing

Integration tests in `tests/api_tests.rs` use an in-memory SQLite database. Test utilities in `tests/common/mod.rs` provide `setup_test_db()` and request helpers. Tests cover health endpoints, card CRUD, and stage transitions.
