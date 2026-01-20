# Fix for "Cannot compact empty turn history" Error

## Problem
The compaction system was triggering when token usage exceeded the threshold, even when `session.turns` was empty, causing the error:
```
Warning: Compaction failed: Cannot compact empty turn history
```

## Root Cause
In `/cli/src/interactive.rs`, the compaction trigger logic was:
```rust
if effective > threshold {
    // Execute compaction without checking if turns exist
}
```

This could happen when:
- High token overhead from system messages before any conversation turns
- First message in session where no complete user→assistant exchange exists yet
- Message parsing fails to create proper turns
- Turn creation returns `None` due to malformed conversation structure

## Fix Applied

### 1. Updated Compaction Trigger Logic
Changed the condition in `/cli/src/interactive.rs` line 870:
```rust
// Before:
if effective > threshold {

// After:  
if effective > threshold && !session.turns.is_empty() {
```

### 2. Added Debug Logging
Added informative debug logging for the edge case:
```rust
} else if effective > threshold {
    // Compaction would trigger but no turns exist to compact
    use tracing::debug;
    debug!(
        effective_tokens = effective,
        threshold = threshold,
        turn_count = session.turns.len(),
        "Compaction threshold exceeded but no conversation turns exist to compact"
    );
}
```

### 3. Created Comprehensive Tests
Added `/cli/tests/empty_turn_compaction_fix_test.rs` with tests for:
- ✅ Compaction NOT triggering when turns are empty (even with high tokens)
- ✅ Compaction triggering when turns exist and tokens are high
- ✅ Defensive logic working correctly in all scenarios

## Result
- The error "Cannot compact empty turn history" will no longer occur
- System gracefully handles high token usage before conversation turns exist
- Debug logging provides visibility into when this condition occurs
- Existing functionality unchanged - compaction still works when turns exist

## Test Results
All three test cases pass:
- `test_compaction_not_triggered_when_turns_empty` ✅
- `test_compaction_triggers_when_turns_exist` ✅  
- `test_defensive_check_logic` ✅

The fix is minimal, defensive, and preserves all existing behavior while preventing the error condition.