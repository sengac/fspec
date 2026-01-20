# AST Research: Token Tracking Fix

## Target File: cli/src/interactive.rs

### Key Functions Identified:

1. **run_agent_stream_with_interruption** (line 582)
   - Contains token tracking logic
   - Lines 837-838 identified as bug location (token accumulation with +=)

2. **Token tracking variables** (around line 666):
   - `turn_input_tokens: u64`
   - `turn_output_tokens: u64`
   - These are updated from `FinalResponse.usage()`

### Bug Location:
Lines 837-838 (approx - need to verify exact line numbers after rig changes):
```rust
session.token_tracker.input_tokens += turn_input_tokens;   // BUG: Should be =
session.token_tracker.output_tokens += turn_output_tokens; // BUG: Should be =
```

### Fix Required:
Replace `+=` with `=` because API returns cumulative context size, not incremental tokens.
