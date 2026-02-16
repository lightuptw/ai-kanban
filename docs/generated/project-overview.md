# LightUp AI Kanban — Project Overview

## Executive Summary

LightUp AI Kanban is an AI-in-the-loop project management tool where AI agents autonomously plan, execute, and deliver work while humans retain control over approvals and stage transitions. Cards flow through a 6-stage pipeline (Backlog > Plan > Todo > In Progress > Review > Done), with AI generating implementation plans, executing work in isolated git worktrees, and streaming real-time progress.

## Project Classification

| Attribute | Value |
|-----------|-------|
| **Project Name** | LightUp AI Kanban |
| **Repository Type** | Multi-part (2 parts) |
| **Repository** | `https://github.com/lightuptw/ai-kanban` |
| **License** | MIT |

| Part | Type | Root Path | Primary Stack |
|------|------|-----------|---------------|
| **backend** | Backend API | `backend/` | Rust 1.75+ / Axum 0.8 / SQLite (sqlx 0.8) / Tokio |
| **frontend** | Web SPA | `frontend/` | React 18 / TypeScript 4.9 / Redux Toolkit / MUI 5 / Vite 5 |

## Architecture Type

**Client-Server with AI Agent Integration**

The system consists of three runtime processes:

| Process | Port | Role |
|---------|------|------|
| Backend (Rust/Axum) | :21547 | REST API, WebSocket, SSE events, static file serving, MCP endpoint |
| Frontend (Vite/React) | :21548 | SPA dev server (production: served by backend from `frontend/dist`) |
| OpenCode CLI | :4096 | AI agent runtime with session management and MCP tool support |

A fourth component — the `kanban-mcp` stdio binary — acts as a stateless HTTP proxy between OpenCode and the backend REST API, enabling AI agents to interact with the board via the Model Context Protocol (MCP).

## Tech Stack Summary

| Layer | Technology | Version |
|-------|-----------|---------|
| Backend Runtime | Rust + Tokio | 1.75+ / edition 2021 |
| Backend Framework | Axum | 0.8 |
| Database | SQLite (WAL mode) | via sqlx 0.8 |
| AI Protocol | MCP (Model Context Protocol) | via rmcp 0.15 |
| Auth | JWT + Argon2id | jsonwebtoken 9 / argon2 0.5 |
| Frontend Framework | React | 18.2 |
| Frontend Language | TypeScript | 4.9 |
| State Management | Redux Toolkit | 1.9.7 |
| UI Components | Material-UI (MUI) | 5.14 |
| Drag-and-Drop | DnD Kit | 6.1 |
| Rich Text Editor | TipTap | 3.19 |
| Build Tool | Vite | 5.0 |
| AI Runtime | OpenCode CLI | latest |

## Key Features

### AI Integration
- Plan generation via MCP tools (AI creates phased subtasks)
- Auto-dispatch queue with configurable per-board concurrency (1-10 or unlimited)
- Git worktree isolation per AI card (branch: `ai/{id}-{slug}`)
- In-app code review with unified diff viewer
- AI question system (select, multi-select, free-text) mid-task
- Auto-detect codebase settings (tech stack, conventions)
- Stop/Resume AI sessions

### Board & Cards
- 6-stage workflow with enforced transition rules
- Drag-and-drop cards within and across stages
- Multi-board support with sidebar navigation
- Priority ordering (high-priority dispatched first)
- Card auto-save with 800ms debounce
- Version history with one-click rollback
- Rich text editor (TipTap) for descriptions
- File attachments, labels, comments

### Real-Time
- WebSocket push for all mutations (20+ event types)
- WebSocket agent logs (live AI activity per card)
- KITT Larson scanner animation on active AI cards
- Sidebar indicators (active AI count per board)

### Auth & Security
- JWT authentication with access + refresh tokens
- Argon2id password hashing
- Service account for MCP tools (localhost-only)
- Default user seeded on first run

## Links to Detailed Documentation

- [Architecture — Backend](./architecture-backend.md)
- [Architecture — Frontend](./architecture-frontend.md)
- [Source Tree Analysis](./source-tree-analysis.md)
- [API Contracts — Backend](./api-contracts-backend.md)
- [Data Models — Backend](./data-models-backend.md)
- [Component Inventory — Frontend](./component-inventory-frontend.md)
- [Integration Architecture](./integration-architecture.md)
- [Development Guide](./development-guide.md)

### Existing Documentation (in `docs/`)
- [Architecture](../architecture.md)
- [API Reference](../api-reference.md)
- [AI Integration](../ai-integration.md)
- [MCP Tools](../mcp-tools.md)
- [Frontend Guide](../frontend.md)
- [Setup Guide](../setup-guide.md)
