# Source Tree Analysis

## Repository Structure

```
ai-kanban/                          # Multi-part repository root
├── README.md                       # Comprehensive project documentation
├── .env.example                    # Environment variable template
├── .gitignore                      # Git ignore rules
├── scripts/                        # Build & dev automation
│   ├── build.sh                    # Production build (frontend + backend)
│   └── dev.sh                      # Development startup (cargo-watch + vite)
│
├── backend/                        # [Part: backend] Rust/Axum API server
│   ├── Cargo.toml                  # Rust dependencies manifest
│   ├── Cargo.lock                  # Locked dependency versions
│   ├── .service-key                # Auto-generated MCP service key
│   ├── kanban.db                   # SQLite database (auto-created)
│   ├── kanban.db-shm               # SQLite shared memory (WAL)
│   ├── kanban.db-wal               # SQLite write-ahead log
│   ├── uploads/                    # File attachment storage
│   ├── migrations/                 # 17 SQLite migration files
│   │   ├── 20260214_001_initial.sql           # cards, subtasks, labels, card_labels, comments + seed labels
│   │   ├── 20260215_001_boards_and_files.sql  # boards table, card_files, cards.board_id
│   │   ├── 20260216_001_subtask_phases.sql    # subtasks.phase, subtasks.phase_order
│   │   ├── 20260217_001_board_position.sql    # boards.position
│   │   ├── 20260218_001_fix_null_board_ids.sql # Default board_id for existing cards
│   │   ├── 20260219_001_settings.sql          # app_settings table
│   │   ├── 20260220_001_agent_logs.sql        # agent_logs table
│   │   ├── 20260221_001_add_ai_agent.sql      # cards.ai_agent column
│   │   ├── 20260222_001_add_card_versions.sql # card_versions table
│   │   ├── 20260223_001_board_settings.sql    # board_settings table
│   │   ├── 20260224_001_board_settings_github_repo.sql # github_repo column
│   │   ├── 20260225_001_auth.sql              # app_secrets, users tables
│   │   ├── 20260226_001_refresh_tokens.sql    # refresh_tokens table
│   │   ├── 20260227_001_ai_questions.sql      # ai_questions table
│   │   ├── 20260228_001_board_settings_concurrency.sql # ai_concurrency
│   │   ├── 20260301_001_card_worktree.sql     # branch_name, worktree_path
│   │   └── 20260302_001_auto_detect_status.sql # auto_detect_status fields
│   ├── src/
│   │   ├── main.rs                 # [ENTRY POINT] Backend server startup
│   │   ├── lib.rs                  # Library root, module exports
│   │   ├── config.rs               # Environment configuration (PORT, DATABASE_URL, etc.)
│   │   ├── bin/
│   │   │   ├── mcp_server.rs       # [ENTRY POINT] kanban-mcp stdio binary
│   │   │   └── verify_db.rs        # Database verification utility
│   │   ├── domain/                 # Domain models & business rules
│   │   │   ├── mod.rs              # Module exports
│   │   │   ├── card.rs             # Card, Subtask, Label, Comment, AgentLog, CardVersion, AiQuestion
│   │   │   ├── stage.rs            # Stage enum with transition validation
│   │   │   └── error.rs            # KanbanError enum, Axum IntoResponse impl
│   │   ├── infrastructure/         # Database initialization
│   │   │   ├── mod.rs              # Module exports
│   │   │   └── db.rs               # SQLite pool setup, WAL mode, migration runner
│   │   ├── auth/                   # Authentication & authorization
│   │   │   ├── mod.rs              # Module exports
│   │   │   ├── jwt.rs              # JWT token creation/verification, signing key mgmt
│   │   │   ├── password.rs         # Argon2 password hashing/verification
│   │   │   ├── middleware.rs       # Auth middleware (JWT + service key validation)
│   │   │   ├── handlers.rs         # Auth endpoints: register, login, refresh, me
│   │   │   └── seed.rs             # Seeds default user + service account on startup
│   │   ├── api/                    # HTTP API layer
│   │   │   ├── mod.rs              # Module exports
│   │   │   ├── routes.rs           # Route definitions, CORS config, static file serving
│   │   │   ├── state.rs            # AppState: db pool, broadcast channel, HTTP client, config
│   │   │   ├── dto/                # Request/response types
│   │   │   │   ├── mod.rs          # Module exports
│   │   │   │   └── cards.rs        # CreateCardRequest, UpdateCardRequest, CardResponse, etc.
│   │   │   └── handlers/           # Route handlers
│   │   │       ├── mod.rs          # Module exports, health_check
│   │   │       ├── cards.rs        # Card CRUD, move, generate-plan, stop-ai, resume, diff, merge, PR, reject
│   │   │       ├── boards.rs       # Board CRUD, reorder
│   │   │       ├── board_settings.rs # Board settings CRUD, auto-detect, clone-repo
│   │   │       ├── subtasks.rs     # Subtask CRUD
│   │   │       ├── comments.rs     # Comment CRUD
│   │   │       ├── labels.rs       # Label listing, card-label add/remove
│   │   │       ├── files.rs        # File upload/download/list/delete
│   │   │       ├── questions.rs    # AI question get/create/answer
│   │   │       ├── settings.rs     # User settings get/set
│   │   │       ├── picker.rs       # OS native directory/file picker
│   │   │       ├── ws.rs           # WebSocket: events stream, per-card logs
│   │   │       └── sse.rs          # WsEvent enum (20+ event types)
│   │   ├── services/               # Business logic & background tasks
│   │   │   ├── mod.rs              # Module exports
│   │   │   ├── card_service.rs     # CardService: CRUD, versions, board queries
│   │   │   ├── ai_dispatch.rs      # AiDispatchService: OpenCode session mgmt
│   │   │   ├── queue_processor.rs  # QueueProcessor: background job, concurrency control
│   │   │   ├── sse_relay.rs        # SseRelayService: OpenCode SSE -> client broadcast
│   │   │   ├── git_worktree.rs     # GitWorktreeService: worktree, diff, merge, PR
│   │   │   └── plan_generator.rs   # PlanGenerator: markdown plans from cards
│   │   └── mcp/                    # MCP (Model Context Protocol) server
│   │       └── mod.rs              # KanbanMcp: 20+ tools, stateless HTTP proxy
│   └── tests/                      # Integration tests
│       ├── api_tests.rs            # Health, card CRUD, stage transitions
│       └── common/
│           └── mod.rs              # Test utilities: in-memory SQLite, request helpers
│
├── frontend/                       # [Part: frontend] React/TypeScript SPA
│   ├── package.json                # Node.js dependencies manifest
│   ├── package-lock.json           # Locked dependency versions
│   ├── tsconfig.json               # TypeScript configuration
│   ├── vite.config.ts              # Vite build config (port 21548, API proxy to :21547)
│   ├── vitest.config.ts            # Vitest test runner config
│   ├── index.html                  # SPA entry HTML
│   ├── public/                     # Static assets
│   └── src/
│       ├── index.tsx               # [ENTRY POINT] React DOM render, Redux Provider
│       ├── App.tsx                  # Root component: store, theme, WebSocket, routes
│       ├── routes.tsx              # Route definitions with AuthGuard, lazy loading
│       ├── constants.ts            # Global constants
│       ├── i18n.ts                 # Internationalization setup
│       ├── vite-env.d.ts           # Vite environment types
│       ├── pages/
│       │   ├── auth/
│       │   │   ├── LoginPage.tsx   # Login form
│       │   │   └── RegisterPage.tsx # Registration form
│       │   └── kanban/             # [CORE UI] Kanban board pages
│       │       ├── KanbanBoard.tsx  # Main board: 6 columns, DnD Kit drag-and-drop
│       │       ├── Column.tsx       # Board column: droppable zone, card count
│       │       ├── KanbanCard.tsx   # Card: drag handle, Larson scanner, stop-AI
│       │       ├── CardDetailDialog.tsx # [LARGEST] Card editor modal (1400+ lines)
│       │       ├── DiffViewer.tsx   # Code diff viewer for review stage
│       │       ├── AgentLogViewer.tsx # Real-time AI log viewer (WebSocket)
│       │       └── BoardSettingsDialog.tsx # Board settings (4 tabs, auto-detect)
│       ├── store/
│       │   └── slices/
│       │       ├── kanbanSlice.ts   # Redux slice: columns, boards, AI status, async thunks
│       │       └── kanbanSlice.test.ts # Redux slice tests
│       ├── services/
│       │   ├── api.ts              # REST API client (all endpoints, auth headers)
│       │   ├── auth.ts             # Auth service (login, register, refresh, tokens)
│       │   └── sse.ts              # WebSocketManager: real-time events, reconnect
│       ├── types/
│       │   ├── kanban.ts           # Card, Board, Subtask, Comment, Label, DiffResult, etc.
│       │   ├── user.ts             # User type
│       │   ├── sidebar.ts          # Sidebar navigation types
│       │   ├── theme.ts            # Theme types
│       │   └── emotion.d.ts        # Emotion CSS-in-JS augmentation
│       ├── components/             # Shared UI components
│       │   ├── AuthGuard.tsx       # Route guard (redirect to /login)
│       │   ├── Async.tsx           # Code-split component wrapper
│       │   ├── Loader.tsx          # Loading spinner
│       │   ├── GlobalStyle.tsx     # Global CSS
│       │   ├── Settings.tsx        # Settings panel
│       │   ├── Footer.tsx          # Footer
│       │   ├── navbar/
│       │   │   ├── Navbar.tsx      # Top navigation bar
│       │   │   ├── NavbarUserDropdown.tsx
│       │   │   └── NavbarNotificationsDropdown.tsx
│       │   └── sidebar/
│       │       ├── Sidebar.tsx     # Sidebar wrapper
│       │       ├── SidebarNav.tsx  # Navigation component
│       │       ├── SidebarNavList.tsx
│       │       ├── SidebarNavListItem.tsx
│       │       ├── SidebarNavSection.tsx
│       │       ├── SidebarFooter.tsx
│       │       ├── dashboardItems.tsx
│       │       └── reduceChildRoutes.tsx
│       ├── layouts/
│       │   └── Dashboard.tsx       # Main layout: sidebar + navbar + content
│       ├── redux/
│       │   ├── store.ts            # Redux store configuration
│       │   └── slices/
│       │       └── counter.ts      # Legacy counter slice
│       ├── hooks/
│       │   ├── useAppDispatch.ts   # Typed dispatch hook
│       │   ├── useAppSelector.ts   # Typed selector hook
│       │   └── useTheme.ts         # Theme hook
│       ├── contexts/
│       │   └── ThemeContext.tsx     # Theme context provider
│       ├── theme/
│       │   ├── index.ts            # Theme factory
│       │   ├── variants.ts         # Light/dark variants
│       │   ├── typography.ts       # Typography config
│       │   ├── breakpoints.ts      # Responsive breakpoints
│       │   ├── components.ts       # MUI component overrides
│       │   └── shadows.ts          # Shadow definitions
│       ├── constants/
│       │   └── stageColors.ts      # Stage-to-color mapping
│       ├── utils/
│       │   ├── createEmotionCache.ts
│       │   └── reportWebVitals.ts
│       └── test/
│           └── setup.ts            # Test setup (happy-dom)
│
├── docs/                           # Existing hand-written documentation
│   ├── architecture.md             # System architecture + diagrams
│   ├── api-reference.md            # REST API endpoints
│   ├── ai-integration.md           # AI workflow + dispatch pipeline
│   ├── mcp-tools.md                # MCP tool reference
│   ├── frontend.md                 # React components + Redux
│   ├── setup-guide.md              # Installation guide
│   └── generated/                  # [THIS OUTPUT] Auto-generated documentation
│
└── .sisyphus/                      # AI agent workspace metadata
    ├── notepads/                   # Agent working notes
    ├── plans/                      # Generated work plans
    └── drafts/                     # Work-in-progress drafts
```

## Critical Folders Summary

| Folder | Purpose | File Count |
|--------|---------|-----------|
| `backend/src/api/handlers/` | All HTTP route handlers | 13 files |
| `backend/src/services/` | Business logic + background tasks | 7 files |
| `backend/src/domain/` | Domain models + business rules | 4 files |
| `backend/src/auth/` | JWT + Argon2 + middleware | 6 files |
| `backend/src/mcp/` | MCP tool server | 1 file |
| `backend/migrations/` | Database schema evolution | 17 files |
| `frontend/src/pages/kanban/` | Core kanban board UI | 7 files |
| `frontend/src/store/slices/` | Redux state management | 2 files |
| `frontend/src/services/` | API client + auth + WebSocket | 3 files |
| `frontend/src/types/` | TypeScript interfaces | 5 files |

## Entry Points

| Binary | Path | Purpose |
|--------|------|---------|
| `kanban-backend` | `backend/src/main.rs` | Main server (API + SSE relay + queue processor) |
| `kanban-mcp` | `backend/src/bin/mcp_server.rs` | stdio MCP server for OpenCode |
| Frontend SPA | `frontend/src/index.tsx` | React application entry |
