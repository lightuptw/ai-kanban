# Setup Guide

## Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| Rust | 1.75+ | Backend compilation |
| Node.js | 18+ | Frontend build and dev server |
| npm | 9+ | Frontend package management |
| opencode CLI | latest | AI agent runtime |

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/lightuptw/ai-kanban.git
cd ai-kanban
```

### 2. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` as needed. Defaults work for local development.

### 3. Install Frontend Dependencies

```bash
cd frontend
npm install
cd ..
```

### 4. Build Backend

```bash
cd backend
cargo build
cd ..
```

This compiles both `kanban-backend` and `kanban-mcp` binaries.

## Running in Development

### Option A: Using the dev script

```bash
./scripts/dev.sh
```

Starts both backend (with cargo-watch) and frontend dev server.

### Option B: Manual startup

Terminal 1 — Backend:
```bash
cd backend
cargo run --bin kanban-backend
```

Terminal 2 — Frontend:
```bash
cd frontend
npm run dev
```

Terminal 3 — OpenCode (in your project directory):
```bash
opencode serve --port 4096
```

### Verify

- Frontend: http://localhost:5173
- Backend API: http://localhost:3000/health
- OpenCode: http://localhost:4096

## Running in Production

### Build

```bash
./scripts/build.sh
```

Or manually:

```bash
cd frontend && npm run build && cd ..
cd backend && cargo build --release && cd ..
```

### Run

```bash
./backend/target/release/kanban-backend
```

The backend serves both the API and the built frontend at `http://localhost:3000`.

## MCP Binary Setup

The `kanban-mcp` binary allows AI agents to interact with the board via MCP tools.

### Build

```bash
cd backend
cargo build --release --bin kanban-mcp
```

### Configure in opencode

Add to `~/.config/opencode/opencode.json`:

```json
{
  "mcp": {
    "kanban": {
      "type": "local",
      "command": ["/absolute/path/to/kanban-mcp"],
      "enabled": true,
      "environment": {
        "KANBAN_API_URL": "http://127.0.0.1:3000"
      }
    }
  }
}
```

Restart opencode after changing config. The backend must be running for MCP tools to work.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Backend HTTP server port |
| `DATABASE_URL` | `sqlite:kanban.db` | SQLite database file path |
| `OPENCODE_URL` | `http://localhost:4096` | OpenCode API endpoint for AI dispatch |
| `FRONTEND_DIR` | `../frontend/dist` | Path to built frontend (production mode) |
| `CORS_ORIGIN` | `http://localhost:5173` | Allowed CORS origins (comma-separated for multiple) |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |
| `KANBAN_API_URL` | `http://127.0.0.1:3000` | MCP binary target (set in opencode config, not .env) |

### Multi-origin CORS

For development across localhost and 127.0.0.1:

```
CORS_ORIGIN=http://localhost:5173,http://127.0.0.1:5173
```

## Database

SQLite database is auto-created and migrations run automatically on startup. The database file location is controlled by `DATABASE_URL`.

Default: `backend/kanban.db`

### Migrations

9 migration files in `backend/migrations/`, run in order:

| Migration | Tables/Changes |
|-----------|---------------|
| `20260214_001_initial.sql` | cards, subtasks, labels, card_labels, comments + seed labels |
| `20260215_001_boards_and_files.sql` | boards, card_files, cards.board_id |
| `20260216_001_subtask_phases.sql` | subtasks.phase, subtasks.phase_order |
| `20260217_001_board_position.sql` | boards.position |
| `20260218_001_fix_null_board_ids.sql` | Default board_id for existing cards |
| `20260219_001_settings.sql` | settings table |
| `20260220_001_agent_logs.sql` | agent_logs table |
| `20260221_001_add_ai_agent.sql` | cards.ai_agent |
| `20260222_001_add_card_versions.sql` | card_versions table |

### WAL Mode

SQLite WAL (Write-Ahead Logging) is enabled in the initial migration for better concurrent read/write performance.

## WSL Notes

When running the backend in WSL with frontend files on `/mnt/c/`:

- Vite HMR (hot module replacement) does not work across the WSL filesystem boundary
- After frontend code changes, restart the Vite dev server manually
- Use `127.0.0.1` instead of `localhost` if DNS resolution is inconsistent

## Troubleshooting

**Backend won't start — "could not determine which binary to run"**: Use `cargo run --bin kanban-backend`. The project has multiple binaries.

**CORS errors in browser**: Ensure `CORS_ORIGIN` includes both `localhost` and `127.0.0.1` variants if needed.

**AI tools return "no such table"**: The MCP binary might be stale. Rebuild with `cargo build --release --bin kanban-mcp`, kill old processes, restart opencode.

**SSE relay not connecting**: Ensure opencode is running with `opencode serve --port 4096`. Check backend logs for SSE connection errors.

**Database locked**: SQLite WAL mode handles most concurrency, but avoid accessing `kanban.db` directly while the backend is running.
