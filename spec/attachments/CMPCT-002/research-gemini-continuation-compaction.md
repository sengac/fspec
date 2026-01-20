# Research: Gemini Continuation + Compaction Handling

## Problem Statement

When using Gemini models with continuation (empty `tool_use` requiring follow-up requests), if compaction is triggered mid-continuation, the current code logs an error and fails the session:

```rust
// Compaction needed during continuation - this is complex to handle
// For now, log and return error. Future: trigger compaction and retry.
error!("Compaction triggered during Gemini continuation - not yet supported");
```

This is treating a recoverable situation as a fatal error.

## Analysis of Other Codebases

### OpenCode (TypeScript)

OpenCode handles compaction **at the turn boundary**, not during continuation:

1. **After each response completes** (`step-finish` event), it checks `SessionCompaction.isOverflow()` 
2. If overflow detected, it sets `needsCompaction = true`
3. After the stream loop completes, it returns `"compact"` signal
4. The **calling code** (`prompt.ts`) then triggers compaction and **loops back** to continue processing

**Key code from `processor.ts`:**
```typescript
case "step-finish":
  // ... update usage stats ...
  if (await SessionCompaction.isOverflow({ tokens: usage.tokens, model: input.model })) {
    needsCompaction = true
  }
  break
// ... later ...
if (needsCompaction) return "compact"
```

**Key code from `prompt.ts`:**
```typescript
const result = await processor.process({ ... })
if (result === "stop") break
if (result === "compact") {
  await SessionCompaction.create({
    sessionID,
    agent: lastUser.agent,
    model: lastUser.model,
    auto: true,
  })
}
continue  // Loop back to process next turn with compacted context
```

**Key insight**: OpenCode doesn't try to handle compaction mid-stream during continuation. It finishes the current response, returns a signal, and the outer loop handles compaction before the next turn.

### VTCode (Rust - Gemini CLI Fork)

VTCode takes an even simpler approach - it **doesn't do compaction at all** during the session:

1. Uses a `ContextManager` with token budget thresholds (Warning at 70%, High at 85%, Critical at 90%)
2. When approaching limits, it **injects guidance** into the system prompt telling the AI to summarize and save artifacts
3. At hard limit (150 messages), it **stops the session** and tells the user to start fresh

**Key code from `context_manager.rs`:**
```rust
pub enum TokenBudgetStatus {
    Normal,      // Below 70%
    Warning,     // 70-85% - start preparing for context handoff
    High,        // 85-90% - active context management needed  
    Critical,    // Above 90% - immediate action required
}

pub(crate) fn pre_request_check(&self, history: &[uni::Message]) -> PreRequestAction {
    let hard_limit = self.agent_config.as_ref()
        .map(|c| c.max_conversation_turns)
        .unwrap_or(150);
    
    if msg_count > hard_limit {
        return PreRequestAction::Stop(format!(
            "Session limit reached ({} messages). Please update artifacts..."
        ));
    }
    // ...
}
```

**Key insight**: VTCode delegates context management to the AI itself via system prompts, rather than trying to compact mid-session.

## Recommended Fix

Based on this research, the fix for our codebase should follow OpenCode's pattern:

1. When compaction is triggered during Gemini continuation, **break out of the continuation loop gracefully**
2. **Return a signal** indicating compaction is needed (similar to OpenCode's `"compact"`)
3. The **main stream loop** should handle compaction and retry the turn

### Implementation Approach

1. Change the error case to a recoverable signal:
```rust
// Instead of:
error!("Compaction triggered during Gemini continuation - not yet supported");
return Err(anyhow::anyhow!("Gemini continuation error: {e}"));

// Do:
warn!("Compaction triggered during Gemini continuation - breaking to compact");
// Save current progress
if !continuation_text.is_empty() {
    handle_final_response(&continuation_text, &mut session.messages)?;
}
// Signal compaction needed
return Ok(StreamLoopResult::NeedsCompaction);
```

2. Add a new return type variant to signal compaction:
```rust
pub enum StreamLoopResult {
    Done,
    NeedsCompaction,
    // ... other variants
}
```

3. Handle the signal in the calling code to trigger compaction and retry.

## Trade-offs

| Approach | Pros | Cons |
|----------|------|------|
| **OpenCode style** (signal + retry) | Clean recovery, no lost work | More complex control flow |
| **VTCode style** (warnings only) | Simpler, no mid-session compaction | Requires manual session management |
| **Current behavior** (error) | Simple | Loses work, bad UX |

The OpenCode approach is recommended as it provides the best user experience while maintaining control over context size.
