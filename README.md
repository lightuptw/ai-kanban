# LightUp AI Kanban

An AI-in-the-loop Kanban board where AI agents autonomously plan, execute, and deliver work — while humans retain control over approvals and stage transitions.

Built with Rust (Axum) + React (MUI) + SQLite + [OpenCode](https://opencode.ai).

## How It Works

Cards flow through a 6-stage pipeline. AI agents generate implementation plans, execute work in isolated git worktrees, and stream real-time progress. Humans decide when to advance cards and approve AI output at the review stage.

```
Backlog --> Plan --> Todo --> In Progress --> Review --> Done
             AI      Queue     AI works       Human
           generates  auto-    in git        reviews
           subtasks  dispatches worktree     diff/PR
```

## Features

### AI Integration
- **Plan generation** — AI analyzes card descriptions and creates phased subtasks via MCP tools
- **Auto-dispatch queue** — Cards in "Todo" are automatically dispatched to AI agents with configurable per-board concurrency (1-10 or unlimited)
- **Git worktree isolation** — Each AI card gets its own branch and worktree under `.lightup-workspaces/`, preventing conflicts during parallel execution
- **In-app code review** — Unified diff viewer with file tree at the review stage; merge, reject with feedback, or create GitHub PRs
- **AI question system** — AI agents can ask users questions mid-task (select, multi-select, or free-text); the card shows a red Larson scanner while waiting for input
- **Auto-detect** — AI analyzes your codebase and auto-fills board settings (tech stack, conventions, testing, infrastructure)
- **Stop / Resume** — Emergency kill switch to cancel runaway AI sessions, with resume capability

### Board & Cards
- **6-stage workflow** with enforced transition rules
- **Drag-and-drop** cards within and across stages
- **Multi-board support** with sidebar navigation
- **Priority ordering** — High-priority cards are dispatched first
- **Card auto-save** with 500ms debounce
- **Version history** with one-click rollback
- **Rich text editor** (TipTap) for card descriptions
- **File attachments** per card
- **Labels** with color-coded tags
- **Comments** with threaded discussion

### Real-Time
- **SSE push for all mutations** — Every card, subtask, comment, label, and board change is pushed to the frontend instantly (20 event types)
- **WebSocket agent logs** — Live streaming of AI activity per card
- **KITT Larson scanner** — Animated indicator on cards while AI is working (color matches the card's current stage)
- **Sidebar indicators** — Active AI session count and Larson scanner per board

### Auth & Security
- **JWT authentication** with access + refresh tokens
- **Argon2id password hashing** with random salts
- **Service account** (`__kanban_ai__`) for MCP tools — auto-generated API key per install, localhost-only access
- **Default user** seeded on first run: `LightUp` / `Spark123`

### Board Settings
4-tab configuration dialog per board:
- **Auto-Detect** — AI-powered codebase analysis with live progress (Larson scanner + timer + collapsible log panel)
- **Basic** — Codebase path, GitHub repo, context notes, reference documents, environment variables, AI concurrency
- **Technical** — Tech stack, infrastructure, environments
- **Conventions** — Code conventions, API conventions, testing requirements, communication patterns

## Architecture

```
+------------------+     REST + SSE      +------------------------+
|  Frontend        | ------------------> |  Backend (Rust/Axum)   |
|  React 18 + MUI  | <----------------- |  Port 21547            |
|  Port 21548      |   SSE /api/events  |                        |
|                  |   WS /ws/logs/{id} |  - REST API (JWT)      |     SQLite
|  - KanbanBoard   |                    |  - SSE broadcast       | -------> kanban.db
|  - CardDialog    |                    |  - WebSocket logs      |
|  - DiffViewer    |                    |  - Queue processor     |
|  - BoardSettings |                    |  - SSE relay           |
|  - Redux store   |                    |  - MCP /mcp endpoint   |
+------------------+                    +-----------+------------+
                                                    |
                                        POST /session, /message
                                        SSE /event
                                                    |
                                                    v
                                        +------------------------+
                                        |  OpenCode CLI          |
                                        |  Port 4096 (serve)     |
                                        |                        |
                                        |  - AI sessions         |
                                        |  - SSE event stream    |
                                        |  - MCP tool client     |
                                        +-----------+------------+
                                                    | stdio
                                                    v
                                        +------------------------+
                                        |  kanban-mcp            |     HTTP
                                        |  (stdio binary)        | -------> Backend API
                                        |  20+ MCP tools         |          (:21547)
                                        +------------------------+
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust 1.75+, Axum 0.8, SQLite (sqlx 0.8), Tokio |
| Frontend | React 18, TypeScript 4.9, Redux Toolkit, MUI 5, DnD Kit 6, TipTap, Vite 5 |
| AI Runtime | [OpenCode](https://opencode.ai) CLI (headless serve mode) |
| AI Protocol | MCP via [rmcp](https://crates.io/crates/rmcp) 0.15 (stdio + streamable HTTP) |
| Auth | JWT (jsonwebtoken), Argon2id (argon2 + password-hash) |
| Build | Cargo (Rust), Vite (frontend), npm |

## Prerequisites

| Tool | Version | Installation |
|------|---------|-------------|
| **Rust** | 1.75+ | [rustup.rs](https://rustup.rs) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Node.js** | 18+ | [nodejs.org](https://nodejs.org) or via nvm: `nvm install 18` |
| **npm** | 9+ | Bundled with Node.js |
| **OpenCode CLI** | latest | [opencode.ai](https://opencode.ai) — `curl -fsSL https://opencode.ai/install \| bash` |
| **Git** | 2.20+ | Required for git worktree features |

### Verify Prerequisites

```bash
rustc --version    # rustc 1.75.0 or higher
cargo --version    # cargo 1.75.0 or higher
node --version     # v18.0.0 or higher
npm --version      # 9.0.0 or higher
opencode --version # any recent version
git --version      # git 2.20 or higher
```

## Quick Start

```bash
# 1. Clone
git clone https://github.com/lightuptw/ai-kanban.git
cd ai-kanban

# 2. Install frontend dependencies
cd frontend && npm install && cd ..

# 3. Build backend (compiles kanban-backend + kanban-mcp binaries)
cd backend && cargo build --release && cd ..

# 4. Start all services (3 terminals or use tmux)

# Terminal 1 - Backend
cd backend && ./target/release/kanban-backend

# Terminal 2 - Frontend (dev mode)
cd frontend && npm run dev

# Terminal 3 - OpenCode (headless server for AI dispatch)
opencode serve --port 4096
```

Open **http://localhost:21548** in your browser.

Login with: **LightUp** / **Spark123**

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `21547` | Backend HTTP server port |
| `DATABASE_URL` | `sqlite:kanban.db` | SQLite database path |
| `OPENCODE_URL` | `http://localhost:4096` | OpenCode API endpoint for AI dispatch |
| `FRONTEND_DIR` | `../frontend/dist` | Built frontend directory (production mode) |
| `CORS_ORIGIN` | `http://localhost:21548,http://127.0.0.1:21548` | Allowed CORS origins (comma-separated) |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

### MCP Configuration for OpenCode

After building the `kanban-mcp` binary, configure it in your OpenCode config (`~/.config/opencode/opencode.json`):

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

The `kanban-mcp` binary reads `KANBAN_API_URL` from its environment (defaults to `http://127.0.0.1:21547`). It acts as a stateless HTTP proxy — all tool calls are forwarded to the backend REST API.

> **Important:** Restart OpenCode after changing the MCP configuration. The backend must be running for MCP tools to work.

### Optional: `.env` File

```bash
cp .env.example .env
# Edit .env - defaults work for local development
```

## Production Build

```bash
# Build everything
./scripts/build.sh

# Or manually:
cd frontend && npm run build && cd ..
cd backend && cargo build --release && cd ..

# Run (serves API + built frontend on a single port)
./backend/target/release/kanban-backend
```

In production, the backend serves the built frontend from `frontend/dist/` at the root path. Only one process needed (plus OpenCode if AI features are desired).

## Project Structure

```
ai-kanban/
├── backend/
│   ├── src/
│   │   ├── api/              # Axum handlers, routes, DTOs, state
│   │   │   ├── handlers/     # boards, cards, subtasks, comments, labels,
│   │   │   │                 # board_settings, questions, sse, ws, picker, files
│   │   │   ├── dto/          # Request/response types
│   │   │   ├── routes.rs     # Route definitions + CORS + static files
│   │   │   └── state.rs      # AppState (db, SSE tx, HTTP client, config)
│   │   ├── auth/             # JWT, Argon2 passwords, middleware, seed
│   │   ├── services/         # AI dispatch, queue processor, SSE relay,
│   │   │                     # plan generator, git worktree
│   │   ├── domain/           # Card, Subtask, Label, Comment, AgentLog models
│   │   ├── mcp/              # MCP server (stateless HTTP proxy, 20+ tools)
│   │   ├── infrastructure/   # Database init + migrations
│   │   ├── bin/              # kanban-mcp binary entry point
│   │   ├── config.rs         # Environment configuration
│   │   ├── lib.rs            # Library root
│   │   └── main.rs           # Backend entry point
│   ├── migrations/           # 17 SQLite migration files
│   └── Cargo.toml
├── frontend/
│   └── src/
│       ├── pages/kanban/     # Board, Card, DiffViewer, BoardSettings,
│       │                     # AgentLogViewer, CardDetailDialog
│       ├── components/       # Sidebar, layouts, guards
│       ├── store/slices/     # Redux kanban slice
│       ├── services/         # API client, SSE handler, auth
│       ├── types/            # TypeScript interfaces
│       ├── theme/            # MUI theme customization
│       └── redux/            # Store setup
├── docs/                     # Detailed documentation
│   ├── architecture.md       # System architecture + data flow diagrams
│   ├── setup-guide.md        # Full installation guide
│   ├── api-reference.md      # REST API endpoints
│   ├── ai-integration.md     # AI workflow + dispatch pipeline
│   ├── mcp-tools.md          # MCP tool reference
│   └── frontend.md           # React components + Redux
├── scripts/
│   ├── build.sh              # Production build script
│   └── dev.sh                # Development startup script
├── .env.example              # Environment variable template
└── .gitignore
```

## Database

SQLite with WAL mode enabled for concurrent reads/writes. The database is auto-created and all 17 migrations run automatically on first startup.

Database file: `backend/kanban.db` (created automatically)

### Key Tables

| Table | Purpose |
|-------|---------|
| `cards` | Work items with AI status, session tracking, worktree paths |
| `subtasks` | Checklist items with phase grouping |
| `boards` | Multiple boards with ordering |
| `board_settings` | Per-board AI context (tech stack, conventions, auto-detect state) |
| `users` | User accounts (Argon2 hashed passwords) |
| `refresh_tokens` | JWT refresh token management |
| `app_secrets` | Service account API keys |
| `agent_logs` | Persisted AI activity logs |
| `card_versions` | Card snapshot history for rollback |
| `ai_questions` | AI-to-user questions with answers |
| `labels` | Color-coded card tags (5 defaults seeded) |
| `comments` | Card discussion threads |

## Card Lifecycle

```
Backlog --[user]--> Plan --[user]--> Todo --[auto]--> In Progress --[AI]--> Review --[user]--> Done
                     |                 |                    |                  |
                     | AI generates    | Queue picks up     | AI works in      | Human reviews
                     | phased subtasks | (priority order)   | git worktree     | diff, merges
                     | via MCP tools   |                    |                  | or rejects
                     |                 | Creates worktree   | Streams logs     |
                     |                 | ai/{id}-{slug}     | via SSE+WS       | Can create
                     |                 | branch             |                  | GitHub PR
```

**AI can only touch**: Plan, Todo, In Progress stages.

**Human decides**: When to move cards to Todo, and when to approve/reject at Review.

## OpenCode Setup

The AI features require [OpenCode](https://opencode.ai) running in **headless serve mode**.

### Install OpenCode

```bash
curl -fsSL https://opencode.ai/install | bash
```

### Start OpenCode Server

```bash
# Must run in your project directory (where opencode.json lives)
cd ai-kanban
opencode serve --port 4096
```

> **Note:** `opencode serve` (headless mode) is different from `opencode` (TUI mode). The kanban backend requires the headless server for API access. You can run both simultaneously — the TUI for interactive use and the serve instance for AI dispatch.

### Configure AI Provider

OpenCode requires an AI provider (Anthropic, Google, OpenAI, etc.). Configure your provider and API keys in `~/.config/opencode/opencode.json`. See [OpenCode documentation](https://opencode.ai) for provider setup.

## WSL Notes

When running on Windows Subsystem for Linux with frontend files on `/mnt/c/`:

- **Vite HMR does not work** across the WSL filesystem boundary — restart the Vite dev server after frontend changes
- Use `127.0.0.1` instead of `localhost` if DNS resolution is inconsistent
- To delete the database: remove `kanban.db`, `kanban.db-shm`, and `kanban.db-wal`

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Backend won't start | Use `cargo run --bin kanban-backend` — the project has multiple binaries |
| CORS errors in browser | Ensure `CORS_ORIGIN` includes both `localhost` and `127.0.0.1` variants |
| AI tools return "no such table" | Rebuild MCP binary: `cargo build --release --bin kanban-mcp`, kill old processes, restart OpenCode |
| SSE relay not connecting | Ensure OpenCode is running with `opencode serve --port 4096` |
| Auto-detect not working | Check that OpenCode serve is running on port 4096 (not just the TUI) |
| Database locked | SQLite WAL handles most concurrency; avoid direct DB access while backend runs |
| OpenCode died overnight | Restart with `opencode serve --port 4096` in a tmux/screen session for persistence |
| MCP tools not responding | Verify backend is running, check `KANBAN_API_URL` in OpenCode config |

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System architecture, data flow, component diagrams |
| [Setup Guide](docs/setup-guide.md) | Detailed installation and configuration |
| [API Reference](docs/api-reference.md) | REST API endpoints and request/response formats |
| [AI Integration](docs/ai-integration.md) | AI workflow, plan generation, dispatch pipeline |
| [MCP Tools](docs/mcp-tools.md) | MCP tool reference and HTTP proxy architecture |
| [Frontend Guide](docs/frontend.md) | React components, Redux store, UI features |

## License

MIT

---

[GitHub](https://github.com/lightuptw/ai-kanban)
