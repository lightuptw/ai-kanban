# Component Inventory — Frontend

## Page Components (pages/)

### Kanban Pages (pages/kanban/)

| Component | File | Lines | Description |
|-----------|------|-------|-------------|
| **KanbanBoard** | KanbanBoard.tsx | ~350 | Main board with 6 columns, DnD Kit drag-and-drop, board header |
| **Column** | Column.tsx | ~130 | Droppable column with card count, stage indicator, add button |
| **KanbanCard** | KanbanCard.tsx | ~240 | Sortable card with Larson scanner, branch chip, stop AI |
| **CardDetailDialog** | CardDetailDialog.tsx | ~1400 | Full card editor modal (most complex component) |
| **DiffViewer** | DiffViewer.tsx | ~170 | Two-pane code diff viewer (file list + diff content) |
| **AgentLogViewer** | AgentLogViewer.tsx | ~385 | Real-time AI log viewer with WebSocket, terminal-style UI |
| **BoardSettingsDialog** | BoardSettingsDialog.tsx | ~880 | 4-tab board configuration modal |

### Auth Pages (pages/auth/)

| Component | File | Description |
|-----------|------|-------------|
| **LoginPage** | LoginPage.tsx | Login form with username/password |
| **RegisterPage** | RegisterPage.tsx | Registration form with full user fields |

## Layout Components (layouts/)

| Component | File | Description |
|-----------|------|-------------|
| **Dashboard** | Dashboard.tsx | Main layout: sidebar + navbar + content area + footer |

## Shared Components (components/)

### Navigation

| Component | File | Description |
|-----------|------|-------------|
| **Navbar** | navbar/Navbar.tsx | Top bar with board name, delete button |
| **NavbarUserDropdown** | navbar/NavbarUserDropdown.tsx | User menu with logout |
| **NavbarNotificationsDropdown** | navbar/NavbarNotificationsDropdown.tsx | Notification bell |
| **Sidebar** | sidebar/Sidebar.tsx | Sidebar wrapper |
| **SidebarNav** | sidebar/SidebarNav.tsx | Navigation tree |
| **SidebarNavList** | sidebar/SidebarNavList.tsx | Navigation list |
| **SidebarNavListItem** | sidebar/SidebarNavListItem.tsx | Individual nav item |
| **SidebarNavSection** | sidebar/SidebarNavSection.tsx | Section with title |
| **SidebarFooter** | sidebar/SidebarFooter.tsx | Sidebar footer |

### Utility Components

| Component | File | Description |
|-----------|------|-------------|
| **AuthGuard** | AuthGuard.tsx | Route guard, redirects to /login if unauthenticated |
| **Async** | Async.tsx | Code-splitting wrapper for lazy-loaded components |
| **Loader** | Loader.tsx | Loading spinner |
| **GlobalStyle** | GlobalStyle.tsx | Global CSS reset/styles |
| **Settings** | Settings.tsx | Settings panel |
| **Footer** | Footer.tsx | Page footer |

## Component Hierarchy

```
App
 ├── Redux Provider
 ├── ThemeProvider
 ├── WebSocketManager (singleton)
 └── BrowserRouter
      └── Routes
           ├── /login → LoginPage
           ├── /register → RegisterPage
           └── / → AuthGuard
                └── Dashboard
                     ├── Sidebar
                     │    ├── SidebarNav
                     │    │    └── SidebarNavSection
                     │    │         └── SidebarNavList
                     │    │              └── SidebarNavListItem
                     │    └── SidebarFooter
                     ├── Navbar
                     │    ├── NavbarUserDropdown
                     │    └── NavbarNotificationsDropdown
                     ├── KanbanBoard ← (main content)
                     │    ├── DndContext (drag-and-drop)
                     │    ├── Column (x6: backlog, plan, todo, in_progress, review, done)
                     │    │    ├── SortableContext
                     │    │    └── KanbanCard (x N)
                     │    │         └── useSortable hook
                     │    └── DragOverlay
                     │         └── KanbanCard (ghost preview)
                     ├── CardDetailDialog (modal, on card click)
                     │    ├── TipTap Editor (description)
                     │    ├── DndContext (subtask reorder)
                     │    ├── DiffViewer (review stage)
                     │    ├── AgentLogViewer (AI stages)
                     │    └── Subtask items (x N)
                     ├── BoardSettingsDialog (modal, on settings click)
                     └── Footer
```

## Hooks Used

| Hook | Source | Usage |
|------|--------|-------|
| `useAppDispatch` | Custom | Typed Redux dispatch |
| `useAppSelector` | Custom | Typed Redux selector |
| `useTheme` | Custom | Theme access |
| `useState` | React | Local component state |
| `useEffect` | React | Side effects, data fetching, WebSocket |
| `useCallback` | React | Memoized callbacks (drag handlers) |
| `useRef` | React | DOM refs, WebSocket refs, debounce timers |
| `useMemo` | React | Phase grouping, display computations |
| `useContext` | React | Theme context |
| `useDroppable` | @dnd-kit | Column drop zones |
| `useSortable` | @dnd-kit | Card sorting |
| `useSensor/useSensors` | @dnd-kit | Drag sensor configuration |

## Animation: KITT Larson Scanner

CSS keyframe animation on KanbanCard when AI is active:
- **Trigger:** `ai_status` in (`planning`, `working`, `dispatched`, `waiting_input`)
- **Visual:** Red sweeping dot (3px height) at card bottom
- **Color:** Matches stage color (plan=blue, in_progress=#376fd0, etc.)
- **Mini variant:** Appears on phase Paper headers in CardDetailDialog

## Stage Colors

| Stage | Color | Hex |
|-------|-------|-----|
| Backlog | Grey | #9e9e9e |
| Plan | Blue | #2196f3 |
| Todo | Orange | #ff9800 |
| In Progress | Blue | #376fd0 |
| Review | Purple | #9c27b0 |
| Done | Green | #4caf50 |
