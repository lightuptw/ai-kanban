# Development Guide

## Prerequisites

| Tool | Version | Installation |
|------|---------|-------------|
| **Rust** | 1.75+ | [rustup.rs](https://rustup.rs) |
| **Node.js** | 18+ | [nodejs.org](https://nodejs.org) or nvm |
| **npm** | 9+ | Bundled with Node.js |
| **OpenCode CLI** | latest | `curl -fsSL https://opencode.ai/install \| bash` |
| **Git** | 2.20+ | Required for worktree features |

### Verify

```bash
rustc --version    # 1.75.0+
cargo --version    # 1.75.0+
node --version     # v18.0.0+
npm --version      # 9.0.0+
opencode --version # any recent
git --version      # 2.20+
```

## Quick Start

```bash
# 1. Clone
git clone https://github.com/lightuptw/ai-kanban.git
cd ai-kanban

# 2. Install frontend deps
cd frontend && npm install && cd ..

# 3. Build backend
cd backend && cargo build && cd ..

# 4. Start (3 terminals)

# Terminal 1 - Backend
cd backend && cargo run --bin kanban-backend

# Terminal 2 - Frontend (dev)
cd frontend && npm run dev

# Terminal 3 - OpenCode (for AI features)
opencode serve --port 4096
```

Open **http://localhost:21548** — Login: **LightUp** / **Spark123**

## Environment Setup

```bash
cp .env.example .env
# Edit .env — defaults work for local development
```

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 21547 | Backend HTTP port |
| `DATABASE_URL` | sqlite:kanban.db | SQLite path |
| `OPENCODE_URL` | http://localhost:4096 | OpenCode API |
| `FRONTEND_DIR` | ../frontend/dist | Built frontend |
| `CORS_ORIGIN` | http://localhost:21548,http://127.0.0.1:21548 | CORS origins |
| `RUST_LOG` | info | Log level |

## Development Scripts

### Automated (scripts/)

```bash
./scripts/dev.sh     # Start backend (cargo-watch) + frontend (vite) together
./scripts/build.sh   # Production build: frontend + backend --release
```

### Manual Commands

**Backend:**
```bash
cd backend
cargo run --bin kanban-backend   # Run server
cargo build --release            # Production build
cargo build --bin kanban-mcp     # Build MCP binary only
cargo test                       # Run tests
RUST_LOG=debug cargo run --bin kanban-backend  # Debug logging
```

**Frontend:**
```bash
cd frontend
npm run dev          # Dev server on :21548
npm run build        # Production build to dist/
npm run lint         # ESLint
npm run type-check   # TypeScript type checking
npm test             # Vitest (run once)
npm run test:watch   # Vitest (watch mode)
```

## MCP Binary Configuration

After building `kanban-mcp`, add to `~/.config/opencode/opencode.json`:

```json
{
  "mcp": {
    "kanban": {
      "type": "local",
      "command": ["/absolute/path/to/backend/target/release/kanban-mcp"],
      "enabled": true,
      "environment": {
        "KANBAN_API_URL": "http://127.0.0.1:21547"
      }
    }
  }
}
```

Restart OpenCode after config changes.

## Production Build & Run

```bash
./scripts/build.sh
# Or manually:
cd frontend && npm run build && cd ..
cd backend && cargo build --release && cd ..

# Single process serves API + frontend:
./backend/target/release/kanban-backend
# → http://localhost:21547
```

## Database

- **Engine:** SQLite (WAL mode)
- **File:** `backend/kanban.db` (auto-created)
- **Migrations:** 17 files in `backend/migrations/`, run automatically on startup
- **Reset:** Delete `kanban.db`, `kanban.db-shm`, `kanban.db-wal`

## Testing

### Backend Tests
```bash
cd backend && cargo test
```
- Integration tests in `tests/api_tests.rs`
- Uses in-memory SQLite for isolation
- Covers: health endpoints, card CRUD, stage transitions

### Frontend Tests
```bash
cd frontend && npm test
```
- Unit tests via Vitest + happy-dom
- Redux slice tests in `store/slices/kanbanSlice.test.ts`
- React component tests via @testing-library/react

## Common Development Tasks

### Add a new API endpoint
1. Define route in `backend/src/api/routes.rs`
2. Create handler in `backend/src/api/handlers/`
3. Add DTO types in `backend/src/api/dto/` if needed
4. Add service method in `backend/src/services/` if needed
5. Update frontend `services/api.ts` with new call
6. Add Redux thunk if needed in `store/slices/kanbanSlice.ts`

### Add a new MCP tool
1. Add tool method in `backend/src/mcp/mod.rs` with `#[tool]` attribute
2. Tool should proxy to an existing REST endpoint
3. Rebuild: `cargo build --release --bin kanban-mcp`
4. Kill old MCP processes, restart OpenCode

### Add a database migration
1. Create file: `backend/migrations/YYYYMMDD_NNN_description.sql`
2. Write SQL (CREATE TABLE, ALTER TABLE, etc.)
3. Migrations run automatically on next startup

### Add a new WebSocket event
1. Add variant to `WsEvent` enum in `backend/src/api/handlers/sse.rs`
2. Broadcast via `state.tx.send(WsEvent::YourEvent {...})` in handler
3. Handle in frontend `services/sse.ts` WebSocketManager
4. Dispatch appropriate Redux action

## WSL Notes

- Vite HMR doesn't work across WSL filesystem boundary — restart dev server manually
- Use `127.0.0.1` instead of `localhost` if DNS issues arise
- DB reset: delete all three files (`kanban.db*`)

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Backend won't start | Use `cargo run --bin kanban-backend` (multiple binaries) |
| CORS errors | Ensure `CORS_ORIGIN` includes both localhost and 127.0.0.1 |
| MCP "no such table" | Rebuild MCP binary, kill old processes, restart OpenCode |
| SSE relay not connecting | Ensure OpenCode running: `opencode serve --port 4096` |
| Auto-detect broken | Check OpenCode serve is running (not just TUI) |
| Database locked | Avoid direct DB access while backend runs (WAL handles concurrency) |
| Stale MCP processes | `ps aux \| grep kanban-mcp`, kill old processes |
