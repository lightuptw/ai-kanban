# Sub-agent Log Visibility — Technical Design

## Problem
When the main AI agent (e.g., "build") spawns sub-agents via `task()` (explore, librarian, oracle, etc.), their logs are silently dropped because `SseRelayService` resolves events to cards via `cards.ai_session_id`, which only stores the parent session ID. Sub-agent sessions have different IDs.

## Root Cause
`sse_relay.rs:130` — `SELECT * FROM cards WHERE ai_session_id = ?` only matches the parent session. Sub-agent events arrive with child session IDs and are dropped at the "Ignoring OpenCode event for unknown session" debug log.

## OpenCode Event Model (Discovered)
- OpenCode emits `session.created` events with `properties.info.parentID` linking sub-agents to their parent session
- Sub-agent session titles follow the pattern: `{description} (@{agent_name} subagent)`
- All events from all sessions flow through the same `/event` SSE endpoint
- `GET /session/{id}/children` API available to list child sessions

## Solution: Three-tier Session Resolution

### Tier 1: Direct Card Match
`SELECT * FROM cards WHERE ai_session_id = ?` (existing behavior, unchanged)

### Tier 2: Session Mapping Lookup
`SELECT card_id FROM session_mappings WHERE child_session_id = ?` — checks pre-registered sub-agent sessions

### Tier 3: Auto-detection
When tiers 1-2 fail:
1. Check `properties.info.parentID` in `session.created` events → resolve parent to card → register mapping
2. Check if parent is itself a known sub-agent session (nested delegation) → resolve transitively
3. Fallback: single active card heuristic (only when exactly one card is working)

## Data Model

### New Table: `session_mappings`
| Column | Type | Description |
|--------|------|-------------|
| child_session_id | TEXT PK | Sub-agent's OpenCode session ID |
| card_id | TEXT FK→cards | Card this sub-agent belongs to |
| parent_session_id | TEXT | Parent session ID |
| agent_type | TEXT? | Agent name extracted from title (explore, librarian, etc.) |
| description | TEXT | Session title/description |
| created_at | TEXT | ISO timestamp |

### Behavior Changes
- Sub-agent events are logged but do NOT trigger card state transitions (no stage moves, no progress updates)
- `agent` field on `agent_logs` is populated from `session_mappings.agent_type` for sub-agent events
- `metadata` JSON includes `_subagent: true` and `_agent_type` for sub-agent logs
- Log content is prefixed with `↳ {agent} | ` for sub-agent events

### New Endpoint
`GET /api/cards/{id}/agent-activity` — returns per-agent event counts and session mappings

## Frontend Changes
- Visual hierarchy: left-border color bars per agent, indentation for sub-agent entries
- Agent filter toggles: clickable chips to show/hide specific agents
- Collapsible groups: consecutive sub-agent logs grouped with summary headers
