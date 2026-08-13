# AST Research: Session and Compaction Integration

## Research Goal
Analyze Session struct and compaction system to understand how to integrate system-reminder persistence through compaction.

## Key Findings

### Session Struct (src/session/mod.rs)

**Current Structure:**
```rust
pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<rig::message::Message>,  // ← System-reminders will live here
    pub turns: Vec<ConversationTurn>,
    pub token_tracker: TokenTracker,
    messages_before_interruption: Option<Vec<rig::message::Message>>,
}
```

**Key Methods:**
- `new()` - Creates session with empty messages Vec
- `switch_provider()` - Clears messages Vec
- Session owns the messages Vec - perfect for adding system-reminder methods

**Integration Point:**
- Add `Session.add_system_reminder()` method
- Add `Session.is_system_reminder()` helper
- NO new fields needed - system-reminders are just Messages in the Vec

### SystemReminderType Enum (src/session/system_reminders.rs)

**Already exists:**
```rust
pub enum SystemReminderType {
    ClaudeMd,
    Environment,
    GitStatus,
    TokenStatus,
}
```

**Helper functions already exist:**
- `create_system_reminder_content(type, content) -> String` - Creates formatted content with type marker
- `extract_type_from_content(content) -> Option<SystemReminderType>` - Extracts type from content
- `is_system_reminder_of_type(msg, type) -> bool` - Checks if message is specific reminder type
- `count_system_reminders_by_type(messages, type) -> usize` - Counts reminders
- `remove_system_reminders_by_type(messages, type) -> Vec<Message>` - Filters out reminders

**Critical for implementation:**
- `add_system_reminder(messages, type, content) -> Vec<Message>` - Already implements the deduplication pattern!
- Currently returns new Vec (functional style)
- Need to adapt to Session's mutable Vec style

### Compaction System (src/agent/compaction.rs)

**ContextCompactor.compact() signature:**
```rust
pub async fn compact<F, Fut>(
    &self,
    turns: &[ConversationTurn],
    llm_prompt: F,
) -> Result<CompactionResult>
```

**Key insight:** Compaction works on `ConversationTurn` structs, NOT directly on messages Vec!

**ConversationTurn structure:**
```rust
pub struct ConversationTurn {
    pub user_message: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub assistant_response: String,
    pub tokens: u64,
    pub timestamp: SystemTime,
    pub previous_error: Option<bool>,
}
```

**Compaction flow:**
1. Detect anchor points in turns
2. Select turns to keep vs summarize
3. Generate summary of summarized turns
4. Return CompactionResult with kept turns + summary

**CompactionResult:**
```rust
pub struct CompactionResult {
    pub kept_turns: Vec<ConversationTurn>,
    pub summary: String,
    pub metrics: CompactionMetrics,
    pub anchor: Option<AnchorPoint>,
}
```

**Critical realization:**
- System-reminders are NOT ConversationTurns
- They are standalone Message structs in Session.messages
- Compaction operates on turns, returns kept turns + summary
- Session must reconstruct messages Vec from compaction result + system-reminders

### Integration Strategy

**Two-phase approach:**

**Phase 1: Session.add_system_reminder() - Use existing function**
```rust
impl Session {
    pub fn add_system_reminder(&mut self, reminder_type: SystemReminderType, content: &str) {
        // Use existing add_system_reminder from system_reminders.rs
        self.messages = add_system_reminder(&self.messages, reminder_type, content);
    }
}
```

**Phase 2: Session.compact_messages() - Protect system-reminders**
```rust
impl Session {
    pub async fn compact_messages<F, Fut>(&mut self, llm_prompt: F) -> Result<()>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // 1. Extract system-reminders from messages Vec
        let (system_reminders, other_messages): (Vec<_>, Vec<_>) =
            self.messages.iter().partition(|msg| is_system_reminder(msg));

        // 2. Compact turns (operates on self.turns, not messages)
        let compactor = ContextCompactor::new();
        let result = compactor.compact(&self.turns, llm_prompt).await?;

        // 3. Reconstruct messages Vec:
        //    - System prompt (if any)
        //    - Compacted summary as user message
        //    - System-reminders (preserved)
        //    - Kept turns converted back to messages
        self.messages.clear();

        // Add summary
        self.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&result.summary))
        });

        // Add system-reminders back (they persist)
        self.messages.extend(system_reminders.cloned());

        // Add kept turns as messages
        for turn in result.kept_turns {
            self.messages.push(Message::User {
                content: OneOrMany::one(UserContent::text(&turn.user_message))
            });
            // ... add assistant response, etc.
        }

        Ok(())
    }
}
```

## Key Insights

1. **No structural changes needed** - Session.messages Vec already holds Messages
2. **Existing helpers** - system_reminders.rs already has most of the logic we need
3. **Compaction is turn-based** - System-reminders (which are Messages, not turns) naturally excluded
4. **Partition pattern** - Use partition() to separate system-reminders before compaction, add back after
5. **Deduplication exists** - add_system_reminder() already implements retain+push pattern

## Implementation Steps

1. ✅ **Add Session.add_system_reminder()** - Thin wrapper around existing function
2. ✅ **Add Session.is_system_reminder()** - Thin wrapper to check if Message is system-reminder
3. ⏳ **Update Session.compact_messages()** - Add partition logic to preserve system-reminders
4. ⏳ **Write tests** - Verify system-reminders persist through compaction

## Files to Modify

- `src/session/mod.rs` - Add add_system_reminder() and compact helper
- `src/session/system_reminders.rs` - Already complete! Just use existing functions
- `src/agent/compaction.rs` - NO changes needed (operates on turns, not messages)
- Tests: Create `tests/system_reminder_persistence_test.rs`
