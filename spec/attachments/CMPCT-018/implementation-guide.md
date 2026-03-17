# CMPCT-018: SessionSearch Scoped Turn Range Queries

## What This Card Does

Adds `start_turn` and `end_turn` parameters to SessionSearch's `show` and `search` actions, enabling the agent to drill into specific DAG node turn ranges without retrieving the full session.

When the agent builds a DAG with `<dag-node turns="45-82">`, future compaction cycles can use `SessionSearch(show, start_turn: 45, end_turn: 82)` to retrieve just those turns for re-summarization.

## Two Deliverables

### 1. Extend Type Definitions

**File:** `codelet/tools/src/session_search/types.rs`  
**Lines:** 18–61 (SessionSearchAction enum)

Add `start_turn` and `end_turn` to both `Show` and `Search` variants:

```rust
Show {
    session_id: Option<String>,
    user_only: Option<bool>,
    max_turns: Option<usize>,
    start_turn: Option<usize>,  // NEW — inclusive, 0-based
    end_turn: Option<usize>,    // NEW — inclusive, 0-based
}

Search {
    query: String,
    context_turns: Option<usize>,
    limit: Option<usize>,
    all_projects: Option<bool>,
    last_hours: Option<u64>,
    last_days: Option<u64>,
    after: Option<String>,
    before: Option<String>,
    start_turn: Option<usize>,  // NEW — restricts search to turn range
    end_turn: Option<usize>,    // NEW — restricts search to turn range
}
```

### 2. Update Tool Definition Schema

**File:** `codelet/tools/src/session_search/mod.rs`  
**Lines:** 68–141 (definition() method)

Add parameters to the JSON schema:

```json
"start_turn": {
    "description": "Start of turn range (inclusive, 0-based) to restrict results (optional for 'show' and 'search' actions)",
    "type": ["integer", "null"]
},
"end_turn": {
    "description": "End of turn range (inclusive, 0-based) to restrict results (optional for 'show' and 'search' actions)",
    "type": ["integer", "null"]
}
```

### 3. Filter Implementation in NAPI Handler

**File:** `codelet/napi/src/session_search_handler.rs`

#### handle_show (lines 283–387)

After loading messages (line ~320) and before applying `max_turns`:

```rust
// Apply turn range filter
let messages = if start_turn.is_some() || end_turn.is_some() {
    let start = start_turn.unwrap_or(0);
    let end = end_turn.unwrap_or(usize::MAX);
    messages.into_iter()
        .enumerate()
        .filter(|(idx, _)| *idx >= start && *idx <= end)
        .map(|(_, msg)| msg)
        .collect()
} else {
    messages
};
```

**Important:** Turn range filter applies BEFORE `max_turns` and `user_only` filters. The turn index is the persisted message index (0-based, after system reminders are excluded).

#### handle_search (lines 128–277)

When searching within a session, skip messages outside the turn range:

```rust
// Inside the message iteration loop
if let (Some(start), _) | (_, Some(end)) = (start_turn, end_turn) {
    let start = start.unwrap_or(0);
    let end = end.unwrap_or(usize::MAX);
    if msg_index < start || msg_index > end {
        continue;
    }
}
```

## Edge Cases

- `start_turn` without `end_turn` → from start_turn to end of session
- `end_turn` without `start_turn` → from beginning to end_turn
- `start_turn > end_turn` → return empty results (don't error)
- `start_turn` beyond session length → return empty results
- Turn range with `search` action: only matches within range are returned, but context_turns can extend slightly outside

## Interaction with Existing Features

- `max_turns` interacts with turn range: first filter by range, then apply max_turns from the end of the filtered set
- `user_only` works within the turn range
- `all_projects` is orthogonal (turn ranges apply per-session)
- `ConditionalTrimmer` (compaction trimming) still applies within the range

## Testing Strategy

- Unit test: show with start_turn=10, end_turn=20 returns only those turns
- Unit test: search with turn range restricts matches
- Unit test: edge cases (empty range, beyond-session range)
- Integration test: build a DAG with turn ranges, then query back specific ranges

## Dependencies

- **CMPCT-017** — DagNodeMeta provides the turn range values the agent will reference
