# AST Research: Stream Loop Compaction Handling

## Research Goal
Understand the current compaction handling in `stream_loop.rs` to implement graceful handling during Gemini continuation.

## Functions Found in stream_loop.rs

```
codelet/cli/src/interactive/stream_loop.rs:45:1:fn  (run_stream_loop - main entry point)
codelet/cli/src/interactive/stream_loop.rs:79:5:fn  (handle_final_response - helper)
codelet/cli/src/interactive/stream_loop.rs:91:5:fn  (debug_file - helper)
codelet/cli/src/interactive/stream_loop.rs:160:5:fn (process_stream_item - stream handler)
codelet/cli/src/interactive/stream_loop.rs:178:5:fn (handle_tool_call - tool executor)
codelet/cli/src/interactive/stream_loop.rs:207:18:fn (closure in middleware)
codelet/cli/src/interactive/stream_loop.rs:241:11:fn (closure)
codelet/cli/src/interactive/stream_loop.rs:274:7:fn  (async closure)
```

## Compaction-Related Code Locations

### Pre-prompt Compaction (lines 299-335)
- Line 299: Hook-based compaction comment
- Line 315: Pre-prompt compaction triggered log
- Line 320: `execute_compaction(session).await` call
- Line 331: `session.token_tracker.reset_after_compaction()` call

### Error Handling for Compaction
- Line 1120-1135: Gemini continuation compaction handling (THE BUG LOCATION)
  - Currently logs error and returns `Err()` when compaction triggered during continuation
  - This is what needs to be fixed

- Line 1170-1191: Main stream error handling for compaction
  - Checks for `PromptCancelled` error
  - Checks `compaction_triggered` flag
  - Triggers recovery compaction if needed

## Key Imports Used
```rust
use crate::compaction_threshold::calculate_usable_context;
use crate::interactive_helpers::execute_compaction;
```

## Current Error Handling (Bug Location - lines 1120-1135)
```rust
// Inside Gemini continuation loop
if is_compaction_cancel {
    // Compaction needed during continuation - this is complex to handle
    // For now, log and return error. Future: trigger compaction and retry.
    error!("Compaction triggered during Gemini continuation - not yet supported");
}

// Clear tool progress callback before returning
set_tool_progress_callback(None);
output.emit_error(&e.to_string());
return Err(anyhow::anyhow!("Gemini continuation error: {e}"));
```

## Fix Required
Change from returning `Err()` to:
1. Save partial `continuation_text` if any
2. Update token tracker with cumulative billing
3. Return a signal (new enum variant) indicating compaction needed
4. Let calling code handle compaction and retry

## Dependencies
- `compaction_threshold::calculate_usable_context` - for threshold calculations
- `interactive_helpers::execute_compaction` - for running compaction
- `token_tracker.reset_after_compaction()` - for post-compaction cleanup
