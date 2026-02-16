# AI Integration

## Overview

AI agents participate in three kanban stages:

| Stage | AI Role | Trigger |
|-------|---------|---------|
| **Plan** | Generates subtasks and phases from card description | User clicks "Generate Plan" |
| **Todo** | Queued for autonomous work execution | User moves card to Todo |
| **In Progress** | AI actively working through the plan | QueueProcessor dispatches |

Human operators control all stage transitions. AI can only move cards to **Review** upon completion.

## Plan Stage — AI Plan Generation

When a user clicks "Generate Plan" on a card in the Plan stage:

1. Backend creates an opencode session via `POST {OPENCODE_URL}/session`
2. Stores `session_id` on the card, sets `ai_status=planning`
3. Sends a prompt containing:
   - Card ID (embedded 5 times to prevent tool calls on wrong cards)
   - Title, description, priority, working directory
   - Linked documents (parsed from JSON array)
   - Attached files (queried from card_files table)
   - Existing subtasks
4. AI uses kanban MCP tools to create subtasks organized by phases
5. Progress visible in real-time via the Agent Log Viewer

### Prompt Safety Rules

The prompt includes mandatory safety rules to prevent rogue behavior:

- ONLY use provided kanban MCP tools
- Do NOT search the filesystem for database files
- Do NOT create, open, or modify .db/.sqlite files
- Do NOT use Python, sqlite3, or shell commands to access databases
- If a MCP tool returns an error, STOP and report — do not work around it

### Configurable AI Agent

Each card has an optional `ai_agent` field (e.g., "bmad-master", "sisyphus"). This is included as an instruction prefix in the prompt sent to opencode, allowing different agent personas for different cards.

## Todo Stage — Queue System

Moving a card to Todo sets `ai_status=queued`. The QueueProcessor background service:

1. Polls every 5 seconds for cards with `ai_status=queued`
2. Counts currently active cards (`ai_status` in: dispatched, working)
3. If active count < `ai_concurrency` setting (default: 1), picks the oldest queued card
4. Generates a `.sisyphus/plans/` markdown file from card data and subtasks
5. Dispatches to opencode via AiDispatchService

### Concurrency Control

The `ai_concurrency` setting (stored in the settings table) controls how many cards can be worked on in parallel. Configurable from the frontend header bar.

### Stuck Card Recovery

If cards remain in `dispatched` status for too long without transitioning to `working`, the QueueProcessor can detect and recover them.

## In Progress Stage — AI Dispatch

AiDispatchService handles the opencode interaction:

1. Creates opencode session (`POST /session`)
2. Saves `session_id` to `cards.ai_session_id`, sets `ai_status=dispatched`
3. Sends the work plan message (`POST /session/{id}/message`) as a background task
4. The message instructs the AI to read the plan file and execute `/start-work`
5. The SSE relay tracks all subsequent progress

## SSE Relay — Event Pipeline

SseRelayService connects to opencode's SSE stream at `GET {OPENCODE_URL}/event` and acts as an event bridge:

```
opencode SSE  ──>  Filter  ──>  Map session→card  ──>  Persist to agent_logs
                                                   ──>  Update card ai_status
                                                   ──>  Broadcast via WebSocket
                                                   ──>  Broadcast via SSE
```

### Event Filtering

| Kept | Skipped |
|------|---------|
| `message.part.delta` (AI text output) | `message.part.updated` (redundant) |
| `message.updated` (with finish field) | `message.updated` (without finish) |
| `session.status` (busy/idle transitions) | `session.diff` (internal state) |
| `session.idle` (completion signal) | `server.connected` (connection noise) |
| `todo.updated` (progress tracking) | `server.heartbeat` (keepalive) |

### State Transitions from Events

| Event | Action |
|-------|--------|
| `session.status` type=busy | Set `ai_status=working`, move card to `in_progress` |
| `session.idle` | Set `ai_status=completed`, move card to `review` |
| `todo.updated` | Update `ai_progress` JSON (completed_todos, total_todos, current_task) |

### Session-to-Card Mapping

The relay extracts `sessionID` from event payloads (nested in various locations) and looks up the corresponding card via: `SELECT * FROM cards WHERE ai_session_id = ?`

## Agent Log Viewer

The frontend AgentLogViewer component:

- Connects via WebSocket to `/ws/logs/{card_id}`
- Displays real-time AI output with auto-scroll
- Shows a color-coded status chip:
  - Blue pulsing = planning
  - Orange pulsing = working
  - Green = completed
  - Red = failed
  - Grey = idle/cancelled
- Small green/grey dot indicates WebSocket connection status
- Only visible for cards in plan, todo, or in_progress stages

## Stop AI

When a user clicks "Stop AI":

1. Frontend calls `POST /api/cards/{id}/stop-ai`
2. Backend validates card has an active AI session
3. Calls `POST {OPENCODE_URL}/session/{session_id}/abort`
4. Sets `ai_status=cancelled` regardless of abort API result
5. Emits SSE `AiStatusChanged` event for instant UI update

The stop button appears:
- Next to "Generate Plan" button (plan stage, when AI is active)
- In the AI Status section (any stage, when AI is active)

## AI Status State Machine

```
idle ──> queued ──> dispatched ──> working ──> completed
                                     │
                                     ├──> failed (on error)
                                     │
                                     └──> cancelled (via Stop AI)
```

| Status | Meaning |
|--------|---------|
| `idle` | No AI activity |
| `queued` | In todo queue, waiting for dispatch |
| `dispatched` | Session created, prompt sent, waiting for AI to start |
| `working` | AI actively processing (session.status = busy) |
| `completed` | AI finished, card moved to review |
| `failed` | Error during dispatch or execution |
| `cancelled` | User stopped the AI session |

## Configuration

| Setting | Source | Default | Description |
|---------|--------|---------|-------------|
| `OPENCODE_URL` | Environment | `http://localhost:4096` | OpenCode API endpoint |
| `KANBAN_API_URL` | MCP env | `http://127.0.0.1:21547` | MCP binary REST API target |
| `ai_concurrency` | Settings table | `1` | Max parallel AI cards |
| `ai_agent` | Per-card field | (none) | Agent persona for the card |

## KITT Larson Scanner

Cards with active AI status (`planning`, `working`, `dispatched`) display a red sweeping dot animation at the bottom — inspired by Knight Rider's KITT. A mini variant appears on phase headers in the card detail dialog.
