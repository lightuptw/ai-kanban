# Frontend Guide

## Overview

The frontend is a React 18 SPA using TypeScript, Redux Toolkit for state management, MUI 5 for components, and DnD Kit for drag-and-drop.

## Tech Stack

| Library | Version | Purpose |
|---------|---------|---------|
| React | 18.2 | UI framework |
| TypeScript | 4.9 | Type safety |
| Redux Toolkit | 1.9 | State management |
| MUI (Material-UI) | 5.14 | Component library |
| DnD Kit | 6.1 | Drag-and-drop |
| Emotion | 11.11 | CSS-in-JS (styled components) |
| Vite | 5.0 | Build tool and dev server |
| React Router | 6.20 | Routing |

## Project Structure

```
frontend/src/
├── App.tsx              # Root component with routing
├── index.tsx            # Entry point, Redux Provider setup
├── pages/
│   └── kanban/
│       ├── KanbanBoard.tsx      # Main board with 6 columns
│       ├── KanbanCard.tsx       # Card component with Larson scanner
│       ├── CardDetailDialog.tsx # Full card editor dialog
│       ├── Column.tsx           # Board column component
│       └── AgentLogViewer.tsx   # Real-time AI log viewer
├── store/
│   └── slices/
│       └── kanbanSlice.ts       # Redux slice (cards, boards, async thunks)
├── services/
│   ├── api.ts                   # REST API client
│   └── sse.ts                   # SSE event handler
├── types/
│   └── kanban.ts                # TypeScript interfaces
├── components/           # Shared UI components
├── layouts/              # Page layouts
├── theme/                # MUI theme customization
├── hooks/                # Custom React hooks
├── contexts/             # React contexts
├── utils/                # Utility functions
└── constants.ts          # App constants
```

## Key Components

### KanbanBoard

The main board view. Renders 6 stage columns (Backlog, Plan, Todo, In Progress, Review, Done) with drag-and-drop support.

- Fetches board data on mount and board selection
- Passes `aiStatus` prop to each KanbanCard
- Header shows board name (editable), AI concurrency selector, delete button
- Sidebar shows board list with drag-to-reorder

### KanbanCard

Individual card rendered in board columns.

- Shows title, priority indicator
- **Larson scanner**: Red sweeping dot animation (CSS-only, 3px height) when `aiStatus` is "planning", "working", or "dispatched"
- Click opens CardDetailDialog
- Draggable via DnD Kit

### CardDetailDialog

Full card editor in a modal dialog. The most complex component.

**Sections (top to bottom):**

1. **Title** — editable inline text field with stage chip
2. **Priority** — dropdown, auto-saves on change
3. **AI Agent** — dropdown (bmad-master, sisyphus, etc.), auto-saves on change
4. **Description** — text area
5. **Generate Plan button** — only in Plan stage; disabled when AI is active; "Stop AI" button appears alongside when AI is running
6. **Subtasks** — grouped by phase, with progress bar, drag-to-reorder, inline add/edit/delete, phase rename
7. **Attached Files** — upload via native dialog, list with download/delete
8. **Linked Documents** — pick files via native dialog
9. **Working Directory** — pick via native dialog
10. **AI Status** — shows status chip (color-coded), progress bar, current task; "Stop AI" button when active
11. **AI Agent Logs** — AgentLogViewer component (only for plan/todo/in_progress stages)
12. **Comments** — list with add/edit/delete
13. **Version History** — collapsible, shows timestamps with restore buttons

**Auto-save**: Priority and AI Agent changes dispatch `updateCard` immediately. No save button — only a "Close" button.

**Mini Larson scanner**: Appears on phase Paper headers when AI is working.

### AgentLogViewer

Real-time AI activity log viewer.

- Connects via WebSocket to `/ws/logs/{card_id}`
- Displays log entries with timestamps and event types
- Auto-scrolls to bottom on new entries
- **AI status chip**: Color-coded with pulse animation
  - Blue pulsing = planning
  - Orange pulsing = working
  - Green = completed
  - Red = failed
  - Grey = idle/cancelled
- **Connection indicator**: Small 8px dot (green = connected, grey = disconnected)

### Column

Board column component. Renders stage header with card count badge, card list, and "Add card" button (Backlog only).

## Redux Store

### kanbanSlice

Central state management for the board.

**State shape:**

```typescript
{
  columns: {
    backlog: Card[],
    plan: Card[],
    todo: Card[],
    in_progress: Card[],
    review: Card[],
    done: Card[]
  },
  boards: Board[],
  selectedBoardId: string,
  loading: boolean,
  error: string | null
}
```

**Async thunks:**

| Thunk | API Call | Description |
|-------|---------|-------------|
| `fetchBoard` | `GET /api/board` | Load all cards grouped by stage |
| `createCard` | `POST /api/cards` | Create new card |
| `updateCard` | `PATCH /api/cards/{id}` | Update card fields |
| `moveCard` | `PATCH /api/cards/{id}/move` | Move card between stages |
| `deleteCard` | `DELETE /api/cards/{id}` | Delete card |
| `fetchBoards` | `GET /api/boards` | Load board list |
| `createBoard` | `POST /api/boards` | Create new board |
| `deleteBoard` | `DELETE /api/boards/{id}` | Delete board |

**Targeted reducers (no full board refetch):**

| Reducer | Purpose |
|---------|---------|
| `updateCardAiStatus` | Updates a single card's `ai_status` in-place (from SSE events) |
| `moveCardInStore` | Moves a card between stage columns in-place |

These reducers prevent the full-board-refetch problem that caused UI flickering when AI status changed.

## API Client (`services/api.ts`)

Centralized fetch wrapper with error handling. All methods return typed responses.

Key methods: `getBoard`, `createCard`, `updateCard`, `moveCard`, `deleteCard`, `generatePlan`, `stopAi`, `getCardVersions`, `restoreCardVersion`, `listBoards`, `getSetting`, `setSetting`.

Uses dynamic base URL: `${window.location.protocol}//${window.location.hostname}:3000` to support both localhost and 127.0.0.1.

## SSE Handler (`services/sse.ts`)

Connects to `GET /api/events` for server-sent events.

| Event | Action |
|-------|--------|
| `AiStatusChanged` | Dispatches `updateCardAiStatus` reducer (targeted, no refetch) |
| `BoardUpdated` | Dispatches `fetchBoard` thunk |
| `CardUpdated` | Dispatches `fetchBoard` thunk |

The SSE handler was optimized to use targeted Redux actions for `AiStatusChanged` events instead of refetching the entire board, which eliminated UI flickering in the CardDetailDialog.

## TypeScript Types (`types/kanban.ts`)

Key interfaces:

| Interface | Fields |
|-----------|--------|
| `Card` | id, title, description, stage, position, priority, working_directory, ai_session_id, ai_status, ai_progress, plan_path, linked_documents, ai_agent, subtask_count, subtask_completed, label_count, comment_count, created_at, updated_at |
| `Subtask` | id, card_id, title, completed, position, created_at, updated_at |
| `Board` | id, name, position, created_at, updated_at |
| `Comment` | id, card_id, author, content, created_at |
| `Label` | id, name, color |
| `AgentLog` | id, card_id, session_id, event_type, agent, content, metadata, created_at |
| `CardVersion` | id, card_id, snapshot, changed_by, created_at |
| `BoardResponse` | backlog, plan, todo, in_progress, review, done (each Card[]) |
| `Stage` | "backlog" \| "plan" \| "todo" \| "in_progress" \| "review" \| "done" |

## Development

```bash
cd frontend
npm run dev      # Start dev server on :5173
npm run build    # Production build to dist/
npm run lint     # ESLint
npm run type-check  # TypeScript type checking
npm run test     # Vitest
```
