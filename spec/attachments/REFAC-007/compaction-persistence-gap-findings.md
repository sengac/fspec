# Compaction Persistence Gap - Investigation Findings

## Executive Summary

Investigation revealed a **CRITICAL gap**: compaction summaries are NEVER persisted to disk, meaning they are completely LOST on session resume.

## Evidence

### The NAPI Binding Exists But Is Never Called

```rust
// codelet/napi/src/persistence/napi_bindings.rs:354
pub fn persistence_set_compaction_state(
    session_id: String,
    summary: String,
    compacted_before_index: u32,
) -> Result<NapiSessionManifest>
```

### Compaction Flow (Current - Broken)

1. `session_compact()` in session_manager.rs:4940 calls `execute_compaction()`
2. `execute_compaction()` in interactive_helpers.rs:171 generates summary via LLM
3. Summary stored in-memory: `session.messages.push(Message::User { content: summary })`
4. **NO CALL to `persistence_set_compaction_state()`** - summary never written to disk!
5. On session resume, compaction summary is completely LOST

### Shell Command to Verify

```bash
# Check if ANY session has compaction state persisted
for f in ~/.fspec/sessions/*.json; do
  compaction=$(jq '.compaction // "null"' "$f" 2>/dev/null)
  if [ "$compaction" != "null" ]; then
    echo "HAS COMPACTION: $(basename $f)"
  fi
done
# Result: ZERO sessions have compaction state!
```

### Impact

- Users who compact sessions lose ALL context when resuming
- Session resume shows only post-compaction messages without summary
- Effectively makes compaction + resume = broken experience

## Code Locations

### Where Compaction Happens (Rust)

- `codelet/napi/src/session_manager.rs:4940` - `session_compact()` NAPI function
- `codelet/cli/src/interactive_helpers.rs:171` - `execute_compaction()` core logic
- `codelet/cli/src/interactive/stream_loop.rs:192` - Hook-triggered compaction

### Where Persistence Should Be Called (But Isn't)

After `execute_compaction()` returns successfully, should call:
```rust
persistence_set_compaction_state(
    session_id,
    result.summary,
    result.metrics.turns_summarized as u32
)?;
```

### Where Restoration Works (If Data Existed)

- `codelet/napi/src/persistence/mod.rs:550` - `get_session_messages()` checks for compaction
- `codelet/napi/src/persistence/napi_bindings.rs:708` - Creates synthetic summary envelope
- This code WORKS - it's just never used because no compaction state is ever persisted!

## Related Files

| File | Purpose |
|------|---------|
| `persistence/napi_bindings.rs:354` | NAPI binding for `persistence_set_compaction_state` |
| `persistence/mod.rs:674` | Core `set_compaction_state()` function |
| `persistence/types.rs:118` | `CompactionState` struct definition |
| `session_manager.rs:4940` | `session_compact()` - should call persistence |
| `interactive/stream_loop.rs:192` | Hook compaction - should call persistence |

## Fix Required

In BOTH compaction pathways (manual and hook-triggered), after `execute_compaction()` succeeds:

1. Call `persistence_set_compaction_state(session_id, summary, boundary_index)`
2. Ensure this is called BEFORE emitting `CompactionComplete` chunk
3. Handle errors appropriately (log but don't fail compaction)

## Test Coverage Needed

1. Compact session → verify `session.compaction` is set in manifest JSON
2. Compact session → resume → verify synthetic summary message appears first
3. Hook-triggered compaction → verify compaction state persisted
4. Emergency compaction → verify compaction state persisted
