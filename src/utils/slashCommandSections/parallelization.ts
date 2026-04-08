export function getParallelizationSection(): string {
  return `## Step 10: Scaling Work with Parallelization Tools

Three tools work together to let you scale beyond single-threaded development:

- **SessionSearch** — Shared memory across all sessions. Search and read any session's conversation history.
- **DeepSearch** — Spawn ephemeral read-only sub-agents to explore large codebases or session histories.
- **AgentManager** — Spawn persistent subordinate agent sessions for parallel work with full tool access.

### When to Use Each Tool

| Tool | Use When | Cost | Persistence |
|------|----------|------|-------------|
| **SessionSearch** | You need to recall what happened in a previous session, find a decision, or pull context from another agent's work | Zero (reads persisted data) | Reads existing data |
| **DeepSearch** | You need to answer a question that requires reading many files or searching across sessions — research tasks | One sub-agent LLM call | Ephemeral (no persistence) |
| **AgentManager** | You need parallel workers doing real work — writing code, running tests, reviewing security — each with full tool access | One LLM session per worker | Full session (searchable) |

### SessionSearch — Cross-Session Memory

SessionSearch gives every agent access to the full conversation history of any session in the project. Use it to:
- **Recall decisions**: Find what was decided in a previous session about architecture, design, or requirements
- **Pull context**: A subordinate agent can read its supervisor's session to understand the current task
- **Search across sessions**: Find all sessions that discussed a topic (e.g., "compaction", "auth refactor")

\`\`\`
# Three actions: recent, search, show

# Discover recent sessions
SessionSearch(action='recent', count=5)

# Search for a topic across all session content (user messages, assistant responses, tool calls)
SessionSearch(action='search', query='authentication', last_hours=24)

# Load a specific session's conversation
SessionSearch(action='show', session_id='<uuid>', max_turns=20)

# Load current session (useful for subordinates reading their own context)
SessionSearch(action='show')

# Scoped search within a turn range (useful for drilling into DAG node references)
SessionSearch(action='show', session_id='<uuid>', start_turn=50, end_turn=80)
\`\`\`

**Key pattern**: Subordinates use SessionSearch to PULL context from their supervisor rather than having context pushed to them. This is explicit and controllable.

### DeepSearch — Ephemeral Research Sub-Agents

DeepSearch spawns a temporary sub-agent with read-only tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) to answer a question over a scoped corpus. The sub-agent explores, synthesizes, and returns a text answer. No persistence, no UI — just a tool call that returns a string.

\`\`\`
# Research a codebase directory
DeepSearch(query='How is authentication handled?', scope=['src/auth/'])

# Research across multiple directories
DeepSearch(query='What validation patterns are used?', scope=['src/commands/', 'src/utils/'])

# Research session history only (no code scope)
DeepSearch(query='What was decided about the database schema?')

# Narrow scope to a single file
DeepSearch(query='Explain the error handling in this file', scope=['src/commands/validate.ts'])

# Control recursion depth (default max_recursion_depth=2, max_depth=50)
DeepSearch(query='Analyze all test patterns', scope=['src/'], max_depth=30)
\`\`\`

**DeepSearch is recursive**: Sub-agents can spawn their own DeepSearch children to decompose large corpora (divide-and-conquer). At max recursion depth, the sub-agent answers directly without further delegation.

**Use DeepSearch for**:
- Exploring unfamiliar codebases before starting work
- Answering "how does X work?" questions that require reading many files
- Finding all sessions where a topic was discussed
- Pre-research before Example Mapping or Event Storming

### AgentManager — Parallel Worker Sessions

AgentManager lets you spawn subordinate agent sessions that run in parallel, each with full tool access (Read, Write, Edit, Bash, Grep, etc.). The spawner is the supervisor — they own the subordinate's lifecycle.

\`\`\`
# Spawn a subordinate with a role (system prompt overlay)
AgentManager(action='spawn', role='You are a security reviewer. Analyze code for vulnerabilities.')
# Returns: { session_id: '<uuid>' }

# Send a task to the subordinate
AgentManager(action='message', session_id='<worker-id>',
  message='Review src/auth/ for SQL injection vulnerabilities and report back')

# Send a message with context from another session's history
AgentManager(action='message', session_id='<worker-id>',
  message='Continue the work from this session',
  context=[{session_id: '<other-session>', start_turn: 0, end_turn: 20}])

# Check on all workers
AgentManager(action='list')

# Get detailed status of a specific worker
AgentManager(action='get_status', session_id='<worker-id>')

# Set or change a role on any session (including your own)
AgentManager(action='set_role', session_id='<my-session-id>',
  role='You are a senior engineer focused on performance optimization')

# Close a subordinate when done
AgentManager(action='close', session_id='<worker-id>')

# Run a time-bounded runtime profiling window (AMGR-017)
# BLOCKS for duration_secs seconds (1..=60, default 10) — this is by design.
# Use to diagnose CPU spikes, runaway loops, or channel backpressure inside the
# stripped production NAPI binary without dtrace/sample.
AgentManager(action='profile', duration_secs=10)
AgentManager(action='profile', duration_secs=5, top_n=10, label_prefix='handle_await_idle')
\`\`\`

### Parallelization Patterns for ACDD

#### Pattern 1: Parallel Research Before Specifying

When starting a complex work unit, use DeepSearch to research in parallel before Example Mapping:

\`\`\`
# Spawn 3 DeepSearch calls to understand the problem space
DeepSearch(query='What existing auth patterns are used?', scope=['src/auth/'])
DeepSearch(query='What sessions discussed authentication requirements?')
DeepSearch(query='What test patterns exist for auth?', scope=['src/__tests__/'])

# Then use results to inform Example Mapping
fspec add-rule AUTH-001 "Must use bcrypt (existing pattern in src/auth/hash.ts)"
\`\`\`

#### Pattern 2: Parallel Workers for Large Implementations

For work units touching multiple components, spawn subordinates for parallel implementation:

\`\`\`
# Supervisor spawns specialized workers
spawn_security = AgentManager(action='spawn', role='Security reviewer')
spawn_tests = AgentManager(action='spawn', role='Test writer')
spawn_docs = AgentManager(action='spawn', role='Documentation updater')

# Send each worker their task
AgentManager(action='message', session_id=spawn_security.session_id,
  message='Review src/auth/ for vulnerabilities. Report findings.')
AgentManager(action='message', session_id=spawn_tests.session_id,
  message='Write tests for the new login flow in src/auth/login.ts')
AgentManager(action='message', session_id=spawn_docs.session_id,
  message='Update API docs for the new auth endpoints')

# Poll status and collect results
AgentManager(action='list')
# Workers message back when done
\`\`\`

#### Pattern 3: Cross-Session Context Sharing

Workers can read each other's sessions and share context:

\`\`\`
# Worker A discovers something relevant to Worker B
# Worker A sends a message to Worker B with context from its own session
AgentManager(action='message', session_id='<worker-b-id>',
  message='I found a dependency you need to know about',
  context=[{session_id: '<worker-a-id>', query: 'dependency discovered'}])

# Worker B can also pull context directly
SessionSearch(action='show', session_id='<worker-a-id>', max_turns=5)
\`\`\`

#### Pattern 4: Supervisor Delegates Investigation

When a supervisor hits an unknown, delegate research without blocking:

\`\`\`
# Supervisor is implementing but needs to understand a dependency
worker = AgentManager(action='spawn', role='Research assistant')
AgentManager(action='message', session_id=worker.session_id,
  message='Investigate how the session persistence layer works in codelet/napi/src/persistence/. I need to know the MessageStore API.',
  context=[{session_id: '<my-session>', query: 'persistence'}])

# Supervisor continues working on other parts
# Worker messages back with findings
\`\`\`

### Important Rules

1. **Subordinates start idle** — They wait for a message before doing anything. Always send a task after spawning.
2. **The spawner owns the lifecycle** — Only the supervisor (or the user) can close a subordinate.
3. **Messages queue, don't interrupt** — If the target is mid-generation, messages wait. Channel capacity is 16.
4. **Workers inherit the supervisor's model** — No model parameter on spawn.
5. **DeepSearch is read-only** — Sub-agents cannot Write, Edit, or modify files. Use AgentManager for write access.
6. **SessionSearch works across all sessions** — Any agent can read any session's history. Use this for context sharing.
7. **Clean up workers when done** — Always close subordinates to free resources.

`;
}
