# MCP Tools

## Overview

The kanban MCP server provides 12 tools for AI agents to interact with the kanban board. It operates as a **stateless HTTP proxy** — every tool call is translated into an HTTP request to the backend REST API.

## Architecture

```
AI Agent (opencode) ──stdio──> kanban-mcp binary ──HTTP──> Backend REST API (:21547) ──> SQLite
```

The MCP binary has no direct database access. This design ensures:

- **Single source of truth**: All writes go through the backend API with its validation and side effects
- **No wrong-database bugs**: The old architecture had the MCP binary opening a different SQLite file when launched from a different working directory
- **Stateless**: The binary only needs `KANBAN_API_URL` to function

## Configuration

In `opencode.json`:

```json
{
  "mcp": {
    "kanban": {
      "type": "local",
      "command": ["/path/to/kanban-mcp"],
      "enabled": true,
      "environment": {
        "KANBAN_API_URL": "http://127.0.0.1:21547"
      }
    }
  }
}
```

The binary reads `KANBAN_API_URL` from environment (defaults to `http://127.0.0.1:21547`).

## Binaries

| Binary | Transport | Use Case |
|--------|-----------|----------|
| `kanban-mcp` | stdio | Used by opencode as MCP tool server |
| Backend `/mcp` endpoint | Streamable HTTP | Direct MCP calls without opencode |

## Tool Reference

### Board Tools (consolidated)

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_board` | `{action?, name?, board_id?}` | Manage boards. Actions: "list" (default), "create" (requires name), "delete" (requires board_id) |

### Card Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_get_board_cards` | `{board_id?}` | Cards grouped by stage (primary overview call) |
| `kanban_get_card` | `{card_id}` | Full card details with subtasks, comments, labels |
| `kanban_create_card` | `{title, description?, stage?, priority?, board_id?}` | Create a card (defaults: stage=backlog, priority=medium) |
| `kanban_update_card` | `{card_id, title?, description?, stage?, priority?, working_directory?, linked_documents?}` | Update card fields |
| `kanban_delete_card` | `{card_id}` | Delete a card |

### Subtask Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_create_subtask` | `{card_id, title, phase?, phase_order?}` | Create subtask (defaults: phase="Phase 1", phase_order=1) |
| `kanban_update_subtask` | `{subtask_id, title?, completed?, position?, phase?, phase_order?}` | Update or check off subtask |
| `kanban_delete_subtask` | `{subtask_id}` | Delete subtask |

### Comment Tool (consolidated)

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_comment` | `{card_id, action?, content?, author?, comment_id?}` | Manage comments. Actions: "add" (default, requires content), "list", "update" (requires comment_id + content), "delete" (requires comment_id) |

### Board Settings Tool (consolidated)

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_board_settings` | `{board_id, action?, codebase_path?, github_repo?, context_markdown?, ...}` | Manage board settings. Actions: "get" (default), "update" (set any settings fields) |

### Question Tool

| Tool | Parameters | Description |
|------|-----------|-------------|
| `kanban_ask_question` | `{card_id, question, question_type?, options?, multiple?}` | Ask user a question, blocks until answered. Types: "select", "multi_select", "text" |

## HTTP Proxy Mapping

Each tool action maps to a REST API call:

| Tool | Action | HTTP Method | Endpoint |
|------|--------|------------|----------|
| `kanban_board` | list | GET | `/api/boards` |
| `kanban_board` | create | POST | `/api/boards` |
| `kanban_board` | delete | DELETE | `/api/boards/{id}` |
| `kanban_get_board_cards` | - | GET | `/api/board?board_id={id}` |
| `kanban_get_card` | - | GET | `/api/cards/{id}` |
| `kanban_create_card` | - | POST | `/api/cards` |
| `kanban_update_card` | - | PATCH | `/api/cards/{id}` |
| `kanban_delete_card` | - | DELETE | `/api/cards/{id}` |
| `kanban_create_subtask` | - | POST | `/api/cards/{card_id}/subtasks` |
| `kanban_update_subtask` | - | PATCH | `/api/subtasks/{id}` |
| `kanban_delete_subtask` | - | DELETE | `/api/subtasks/{id}` |
| `kanban_comment` | list | GET | `/api/cards/{card_id}/comments` |
| `kanban_comment` | add | POST | `/api/cards/{card_id}/comments` |
| `kanban_comment` | update | PATCH | `/api/comments/{id}` |
| `kanban_comment` | delete | DELETE | `/api/comments/{id}` |
| `kanban_board_settings` | get | GET | `/api/boards/{id}/settings` |
| `kanban_board_settings` | update | PUT | `/api/boards/{id}/settings` |
| `kanban_ask_question` | - | POST | `/api/cards/{card_id}/questions` |

## IntoKanbanApiUrl Trait

The MCP server uses a generic `IntoKanbanApiUrl` trait to accept either a `String` URL or a `SqlitePool` (which reads from env). This allows the same `KanbanMcp::new()` call to work in both the stdio binary (passing a URL string) and the embedded `/mcp` endpoint in main.rs (passing the pool).

## Testing the MCP Binary

Send JSON-RPC messages via stdin:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' \
  | KANBAN_API_URL=http://127.0.0.1:21547 ./target/release/kanban-mcp
```

## Troubleshooting

**"no such table" errors**: The old MCP binary accessed SQLite directly. If opencode launched it from a different working directory, it created/used a different database file. Solution: use the new HTTP proxy binary and restart opencode.

**Stale MCP processes**: opencode caches MCP processes. After rebuilding the binary, kill old processes (`ps aux | grep kanban-mcp`) and restart opencode.

**MCP not responding**: Ensure the backend is running on the port specified in `KANBAN_API_URL`.
