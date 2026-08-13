# Scaling Work: AgentManager, SessionSearch & DeepSearch

Three tools work together to scale beyond single-threaded development:

## AgentManager — Parallel Worker Sessions

Spawn subordinate AI agents with full tool access. Each worker inherits the supervisor's model and runs independently:

- **spawn** — Create a new worker session with an optional role (e.g., "security reviewer")
- **message** — Send tasks to workers, with optional context references from other sessions
- **list / get_status** — Monitor worker progress
- **await_idle** — Block until workers finish (instead of polling)
- **close** — Terminate workers when done

```
AgentManager(action='spawn', role='Security reviewer')
# → { session_id: 'abc-123' }

AgentManager(action='message', session_id='abc-123',
  message='Review src/auth/ for vulnerabilities')

AgentManager(action='await_idle', session_id='abc-123')
# Blocks until the worker finishes

AgentManager(action='close', session_id='abc-123')
```

## SessionSearch — Cross-Session Memory

Search and view conversation history across all sessions. Workers use SessionSearch to PULL context from their supervisor:

- **recent** — List recent sessions for discovery
- **search** — Keyword search with regex across all content (user inputs, responses, tool calls)
- **show** — Load a specific session's conversation

```
SessionSearch(action='recent', count=5)
# → List of recent sessions with timestamps

SessionSearch(action='search', query='authentication', last_hours=24)
# → Matches with surrounding context

SessionSearch(action='show', session_id='abc-123', max_turns=20)
# → Conversation history for drill-down
```

## DeepSearch — Ephemeral Research Sub-Agents

Spawn a read-only sub-agent that explores a scoped corpus (code files or session history) and returns a synthesized answer:

```
DeepSearch(query='How is authentication handled?', scope='src/auth/')
# → Sub-agent explores the directory and returns findings

DeepSearch(query='What was decided about the database schema?')
# → Searches session history only (no code scope)
```

## How They Work Together

| Tool | Use When | Persistence |
|------|----------|-------------|
| **SessionSearch** | Recall decisions, pull context from another agent | Reads existing data |
| **DeepSearch** | Answer research questions requiring many file reads | Ephemeral (no persistence) |
| **AgentManager** | Parallel workers doing real work — writing code, running tests | Full session (searchable) |

**Typical pattern:**

1. Supervisor spawns workers via AgentManager
2. Workers use SessionSearch to pull context from the supervisor
3. Workers use DeepSearch for codebase research
4. Supervisor uses `await_idle` to wait for results
5. Workers close when done

This enables factory-scale parallelism: one agent implements a feature while another reviews security, all sharing context through SessionSearch.
