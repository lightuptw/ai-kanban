# Architecture — Frontend

## Overview

The frontend is a React 18 Single-Page Application (SPA) built with TypeScript, Redux Toolkit for state management, Material-UI (MUI) 5 for the component library, and DnD Kit for drag-and-drop. Vite 5 handles development serving and production builds.

## Technology Stack

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| Framework | React | 18.2 | UI rendering |
| Language | TypeScript | 4.9 | Type safety |
| State Mgmt | Redux Toolkit | 1.9.7 | Centralized state |
| UI Library | Material-UI (MUI) | 5.14 | Component library |
| CSS-in-JS | Emotion | 11.11 | Styled components |
| Drag & Drop | DnD Kit | 6.1 | Kanban card/subtask reordering |
| Rich Text | TipTap | 3.19 | Card description editor |
| Routing | React Router | 6.20 | Client-side routing |
| Build Tool | Vite | 5.0 | Dev server + bundler |
| Testing | Vitest + Testing Library | 4.0 / 16.3 | Unit + component tests |
| i18n | i18next + react-i18next | 22.5 / 12.3 | Internationalization |

## Architecture Pattern

**Single-Page Application with Redux unidirectional data flow + WebSocket real-time sync.**

```
┌─────────────────────────────────────────────────────────────────┐
│  App.tsx (Root)                                                  │
│  ├── Redux Provider (store)                                      │
│  ├── Theme Provider (MUI theme)                                  │
│  ├── WebSocketManager (singleton, real-time events)              │
│  └── Routes (React Router)                                       │
│       ├── /login → LoginPage                                     │
│       ├── /register → RegisterPage                               │
│       └── / → AuthGuard → Dashboard                              │
│            ├── Sidebar (board list, navigation)                   │
│            ├── Navbar (board name, actions)                       │
│            └── KanbanBoard (main content)                        │
│                 ├── Column (x6 stages)                           │
│                 │   └── KanbanCard (x many)                      │
│                 ├── CardDetailDialog (modal)                     │
│                 │   ├── DiffViewer                                │
│                 │   └── AgentLogViewer                            │
│                 └── BoardSettingsDialog (modal)                   │
└─────────────────────────────────────────────────────────────────┘
```

## State Management

### Redux Store Shape

```typescript
{
  kanban: {
    columns: {
      backlog: Card[],
      plan: Card[],
      todo: Card[],
      in_progress: Card[],
      review: Card[],
      done: Card[]
    },
    loading: boolean,
    error: string | null,
    selectedCardId: string | null,
    boards: Board[],
    activeBoardId: string | null,
    boardsLoading: boolean,
    autoDetectStatus: Record<string, { status: string, startedAt?: string }>
  }
}
```

### Async Thunks

| Thunk | API Call | Description |
|-------|---------|-------------|
| `fetchBoard` | GET /api/board | Load all cards grouped by stage |
| `fetchBoards` | GET /api/boards | Load board list |
| `createBoard` | POST /api/boards | Create new board |
| `updateBoard` | PATCH /api/boards/{id} | Update board |
| `reorderBoard` | PATCH /api/boards/{id}/reorder | Reorder board |
| `deleteBoard` | DELETE /api/boards/{id} | Delete board |
| `createCard` | POST /api/cards | Create new card |
| `updateCard` | PATCH /api/cards/{id} | Update card fields |
| `moveCard` | PATCH /api/cards/{id}/move | Move card between stages |
| `deleteCard` | DELETE /api/cards/{id} | Delete card |

### Targeted Reducers (No Full Refetch)

| Reducer | Purpose |
|---------|---------|
| `updateCardFromSSE` | Updates a single card in-place from WebSocket |
| `removeCardFromSSE` | Removes a card from its column |
| `updateCardAiStatus` | Updates AI status/progress fields in-place |
| `moveCardInStore` | Moves card between stage columns |
| `optimisticMoveCard` | Immediately updates UI on drag |
| `revertMoveCard` | Reverts failed drag-and-drop |
| `updateBoardFromSSE` | Updates board in list |
| `removeBoardFromSSE` | Removes board from list |

These targeted reducers prevent full-board refetch on every event, eliminating UI flickering.

## Real-Time Architecture

### WebSocketManager (services/sse.ts)

Singleton class initialized in `App.tsx`. Connects to `/ws/events?token=<jwt>`.

**Event handling:**

| Event | Redux Action |
|-------|-------------|
| `cardCreated`, `cardUpdated` | `updateCardFromSSE` |
| `cardMoved` | `moveCardInStore` |
| `cardDeleted` | `removeCardFromSSE` |
| `aiStatusChanged` | `updateCardAiStatus` |
| `subtaskCreated/Updated/Toggled` | `updateCardSubtaskFromWS` |
| `subtaskDeleted` | `removeCardSubtaskFromWS` |
| `commentCreated/Updated` | `updateCardCommentFromWS` |
| `commentDeleted` | `removeCardCommentFromWS` |
| `boardCreated/Updated` | `updateBoardFromSSE` |
| `boardDeleted` | `removeBoardFromSSE` |
| `autoDetectStatus` | `setAutoDetectStatus` |
| `questionCreated/Answered` | `updateCardAiStatus` |

**Reconnection:** Exponential backoff (max 30s). Reconnects on auth token refresh.

### AgentLogViewer WebSocket

Separate per-card WebSocket connection to `/ws/logs/{card_id}`. Replays existing logs on connect, then streams new ones. Dark terminal UI with agent color coding.

## Drag-and-Drop

### Board-Level (KanbanBoard.tsx)

- **Library:** @dnd-kit/core + @dnd-kit/sortable
- **Collision:** Custom strategy combining `pointerWithin` and `rectIntersection`
- **Sensors:** PointerSensor (8px activation), KeyboardSensor (accessibility)
- **Optimistic:** UI updates immediately on `dragOver`, confirmed on `dragEnd`
- **Revert:** On API failure, `revertMoveCard` restores previous position
- **Position:** Fractional positioning (1000, 2000, 1500 between)

### Subtask-Level (CardDetailDialog.tsx)

- DndContext within the card detail modal
- Subtask reordering with drag handles
- Position updates sent to API on drop

## Key Components

### CardDetailDialog (1400+ lines)

The most complex component. Sections:
1. Title (inline edit, auto-save 800ms)
2. Priority dropdown
3. AI Agent selector
4. Description (TipTap rich text)
5. Generate Plan button (Plan stage only)
6. Subtasks (phased, draggable, inline CRUD)
7. Attached Files (upload/download)
8. Linked Documents (file picker)
9. Working Directory (directory picker)
10. AI Status (progress bar, current task, stop/resume)
11. AI Questions (text/select/multi-select)
12. Code Review (DiffViewer, merge/reject/PR)
13. Agent Logs (AgentLogViewer)
14. Comments (CRUD)
15. Version History (restore)

### KanbanCard

- Larson scanner animation (CSS keyframes) when AI is active
- Stage-colored indicator bar
- Stop AI button overlay
- Branch name chip

### BoardSettingsDialog (879 lines)

4-tab configuration:
1. **Auto-Detect:** Codebase path, GitHub URL, clone, AI analysis with live progress
2. **Basic:** AI concurrency, context notes, documents, env vars
3. **Technical:** Tech stack, communication, environments, infrastructure
4. **Conventions:** Code, testing, API conventions

## API Client (services/api.ts)

Centralized fetch wrapper. Base URL from `VITE_API_URL` env or dynamic `{protocol}//{hostname}:21547`. Automatic JWT header injection. Auto-refresh on 401 responses.

## Auth Flow

1. Login → store JWT + refresh token in localStorage
2. API calls include Bearer token
3. On 401 → auto-refresh token
4. On refresh failure → redirect to /login
5. AuthGuard component protects all routes except /login and /register

## Theme System

MUI 5 theme with Emotion CSS-in-JS. Light/dark variants. Stage colors:

| Stage | Color |
|-------|-------|
| Backlog | Grey (#9e9e9e) |
| Plan | Blue (#2196f3) |
| Todo | Orange (#ff9800) |
| In Progress | Blue (#376fd0) |
| Review | Purple (#9c27b0) |
| Done | Green (#4caf50) |

## Testing

- **Framework:** Vitest 4.0 + happy-dom
- **React Testing:** @testing-library/react 16.3
- **Test files:** `kanbanSlice.test.ts` (Redux slice tests), `test/setup.ts`
- **Commands:** `npm test` (run), `npm run test:watch` (watch mode)
