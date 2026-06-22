# Uteke MCP Integration — AI Agent Setup

Uteke exposes persistent semantic memory to AI coding agents via MCP (Model Context Protocol).

## Quick Start

### 1. Start uteke-serve (MCP over HTTP — recommended)

```bash
uteke-serve --port 8767 &
```

This gives you:
- MCP endpoint at `http://127.0.0.1:8767/mcp`
- Shared vector index (all agents see same data)
- Recall cache (~42ms vs ~3s cold start)
- Auto-linking (cosine edges on every remember)

### Alternative: stdio mode (per-agent isolated index)

```bash
cargo install --path uteke/crates/uteke-mcp --force
```

⚠️ **Warning**: stdio MCP has its own vector index. Memories inserted via
stdio won't be immediately searchable via uteke-serve HTTP. Use stdio only
if you don't run uteke-serve.

### 2. Configure your AI agent

---

## Claude Code

**Project-level** (`.mcp.json` in project root):
```json
{
  "mcpServers": {
    "uteke": {
      "type": "url",
      "url": "http://127.0.0.1:8767/mcp"
    }
  }
}
```

**Or stdio fallback** (if Claude Code doesn't support URL type):
```json
{
  "mcpServers": {
    "uteke": {
      "command": "uteke-mcp",
      "args": []
    }
  }
}
```

## Pi Dev / OpenCode / Any MCP client

Point to the HTTP endpoint:
```
MCP Server URL: http://127.0.0.1:8767/mcp
```

Or use stdio:
```json
{
  "mcp": {
    "servers": {
      "uteke": {
        "command": "uteke-mcp",
        "args": []
      }
    }
  }
}
```

## Any MCP-compatible agent (HTTP mode)

Instead of stdio, use the HTTP endpoint on uteke-serve:

```
POST http://127.0.0.1:8767/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

---

## Available Tools

| Tool | Description |
|------|-------------|
| `uteke_remember` | Store a memory (content, tags, namespace, type) |
| `uteke_recall` | Semantic search — returns ranked results by meaning |
| `uteke_list` | List memories (filter by tag, namespace) |
| `uteke_forget` | Delete a memory by ID |
| `uteke_stats` | Get memory statistics (count, tags, tiers) |

## How Agents Use It

```
Agent starts task →
  uteke_recall("project conventions, patterns") →
    Gets relevant memories (semantic match)
  → Applies context to current task
  → Learns something new →
    uteke_remember("discovered: API uses X pattern") →
      Stored permanently, searchable by next agent session
```

## Benefits

- **Persistent**: Memory survives across sessions, restarts, different agents
- **Semantic**: Search by meaning, not just keywords
- **Auto-linked**: Similar memories connected automatically
- **Namespaced**: Isolate per project or share globally
- **Local-first**: All data stays on your machine
