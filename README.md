# AI Kanban Board

An AI-in-the-loop Kanban board where AI agents actively participate in planning, executing, and reviewing work items.

## Overview

AI Kanban Board integrates [OpenCode](https://opencode.ai) AI agents directly into the card workflow. AI generates implementation plans from card descriptions, autonomously executes work through a managed queue, and streams real-time progress logs to the frontend. Human operators retain control over stage transitions while AI handles the heavy lifting.

## Key Features

- **6-stage workflow** — Backlog, Plan, Todo, In Progress, Review, Done with enforced transition rules
- **AI plan generation** — Generates phased subtasks from card descriptions using MCP tools
- **AI work dispatch** — Queued cards are automatically dispatched to AI agents with configurable concurrency
- **Real-time log streaming** — WebSocket-powered live AI activity logs per card
- **KITT Larson scanner** — Red sweeping animation on cards while AI is actively working
- **Card version history** — Auto-snapshots before updates with one-click rollback
- **Auto-save** — Priority and agent changes save immediately without a save button
- **Stop AI** — Emergency kill switch to cancel runaway AI sessions
- **MCP integration** — Model Context Protocol server as stateless HTTP proxy
- **Drag-and-drop** — Cards reorderable within and across stages
- **Multi-board support** — Multiple boards with drag-to-reorder sidebar

## Architecture

```
Frontend (React :5173) ──> REST API (Rust/Axum :3000) ──> SQLite
AI Agent (opencode)    ──> MCP (stdio binary)          ──> REST API (:3000) ──> SQLite
opencode SSE           ──> SSE Relay                   ──> WebSocket ──> Frontend
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Axum 0.8, SQLite (sqlx 0.8), Tokio |
| Frontend | React 18, TypeScript, Redux Toolkit, MUI 5, DnD Kit 6 |
| AI Integration | OpenCode API, MCP (rmcp 0.15), SSE relay |
| Build | Cargo, Vite 5 |

## Quick Start

**Prerequisites:** Rust 1.75+, Node.js 18+, [opencode CLI](https://opencode.ai)

```bash
# 1. Clone and configure
git clone https://github.com/lightuptw/ai-kanban.git
cd ai-kanban
cp .env.example .env

# 2. Start development servers
cd frontend && npm install && npm run dev &
cd backend && cargo run --bin kanban-backend &

# 3. Start opencode (in your project directory)
opencode serve --port 4096
```

Backend serves at `http://localhost:3000`, frontend at `http://localhost:5173`.

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System architecture, data flow, component diagram |
| [Setup Guide](docs/setup-guide.md) | Installation, configuration, environment variables |
| [API Reference](docs/api-reference.md) | REST API endpoints, request/response formats |
| [AI Integration](docs/ai-integration.md) | AI workflow, plan generation, dispatch pipeline, SSE relay |
| [MCP Tools](docs/mcp-tools.md) | MCP server tools, configuration, HTTP proxy architecture |
| [Frontend Guide](docs/frontend.md) | React components, Redux store, UI features |

## Project Structure

```
ai-kanban/
├── backend/
│   ├── src/
│   │   ├── api/            # Axum handlers, routes, DTOs, state
│   │   ├── services/       # AI dispatch, queue processor, SSE relay
│   │   ├── domain/         # Card, Subtask, Label, Comment models
│   │   ├── mcp/            # MCP server (HTTP proxy to REST API)
│   │   ├── infrastructure/ # Database initialization
│   │   ├── bin/            # kanban-mcp binary entry point
│   │   └── main.rs         # Backend entry point
│   └── migrations/         # SQLite schema migrations (9 files)
├── frontend/
│   └── src/
│       ├── pages/kanban/   # Board, Card, Dialog, LogViewer components
│       ├── store/slices/   # Redux kanban slice
│       ├── services/       # API client, SSE handler
│       └── types/          # TypeScript interfaces
├── docs/                   # Documentation
└── scripts/                # build.sh, dev.sh
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Backend server port |
| `DATABASE_URL` | `sqlite:kanban.db` | SQLite database path |
| `OPENCODE_URL` | `http://localhost:4096` | OpenCode API endpoint |
| `FRONTEND_DIR` | `../frontend/dist` | Built frontend directory (production) |
| `CORS_ORIGIN` | `http://localhost:5173` | Allowed CORS origins (comma-separated) |
| `RUST_LOG` | `info` | Log level filter |
| `KANBAN_API_URL` | `http://127.0.0.1:3000` | MCP binary target API (env for kanban-mcp) |

## License

MIT

---

[GitHub](https://github.com/lightuptw/ai-kanban)
