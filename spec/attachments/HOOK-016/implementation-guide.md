# HOOK-016: Session & Notification Lifecycle Integration

## What This Card Delivers

Wires the lifecycle hook engine into the agent loop for non-tool events. After this card is complete:
- session_start hooks fire at session creation, injecting context as system messages
- session_end hooks fire on session cleanup with termination reason
- user_prompt_submit hooks can block prompts before the agent sees them
- notification hooks fire via a global engine outside the agent loop
- Multiple commands in a hook group execute sequentially

## Depends On

- **HOOK-014** — config types and compiled hooks
- **HOOK-015** — execution engine and output interpretation

## Can Run In Parallel With

- **HOOK-017** (Tool Use Integration) — independent integration point

## Integration Points in session_manager.rs

### BackgroundSession Changes

Add a new field:
```rust
struct BackgroundSession {
    // ... existing fields ...
    lifecycle_hooks: Option<LifecycleHookEngine>,  // NEW
}
```

The engine is created during `SessionManager::create_session_with_id()`:
1. Resolve project root
2. Load & merge config from `~/.fspec/fspec-hooks.json` + `spec/fspec-hooks.json`
3. If any agent lifecycle events found → `Some(LifecycleHookEngine::new(...))`
4. If none → `None`

### session_start — After session creation, before first prompt

In `agent_loop()`, insert BEFORE the main loop starts:

```
// After session is fully set up, before entering input wait loop
if let Some(ref hooks) = session.lifecycle_hooks {
    let outcome = hooks.run_session_start(SessionStartTrigger::Startup).await;
    // Inject outcome.additional_context as system messages
    // Display outcome.messages
}
```

For resumed sessions, use `SessionStartTrigger::Resume`.

### session_end — On session cleanup/destroy

In `destroy_session()` or the agent_loop cleanup path:

```
if let Some(ref hooks) = session.lifecycle_hooks {
    let outcome = hooks.run_session_end(reason).await;
    // Display outcome.messages
    // Cleanup global notification engine
}
```

Reasons: `"completed"`, `"exit"`, `"cancelled"`, `"error"`

### user_prompt_submit — After input received, before agent processes it

In `agent_loop()`, in the `tokio::select!` input branch, AFTER receiving user input but BEFORE calling `run_agent_stream_internal()`:

```
if let Some(ref hooks) = session.lifecycle_hooks {
    let outcome = hooks.run_user_prompt_submit(&prompt_text).await;
    if !outcome.allow_prompt {
        // Emit block reason to output buffer
        // Skip this prompt, continue to next input wait
        continue;
    }
    // Inject outcome.additional_context as system messages
}
```

**Key behavior**: If prompt is blocked, the agent loop returns to waiting for input. The agent never sees the prompt.

### notification — Global engine via OnceLock

Notifications can fire from outside the agent loop (e.g., permission prompts, idle timeouts). Use a global pattern:

```rust
static NOTIFICATION_ENGINE: OnceLock<RwLock<Option<LifecycleHookEngine>>> = OnceLock::new();

// Set during session_start
pub fn set_notification_engine(engine: Option<LifecycleHookEngine>) { ... }

// Called from anywhere
pub async fn run_notification_hook(type: &str, title: &str, message: &str) {
    if let Some(engine) = get_notification_engine() {
        engine.run_notification(type, title, message).await;
    }
}
```

### Sequential Command Execution

Within a single hook group, commands execute sequentially (not concurrently):
```rust
for command in &group.commands {
    let result = execute_command(command, &payload, &env).await;
    // Accumulate results
}
```

## Context Injection

When hooks return `additional_context`, these should be added as system messages to the conversation. The exact mechanism depends on how messages are managed in the session:

- For `session_start`: inject before first user message
- For `user_prompt_submit`: inject right before the prompt is processed
- Use the existing `inner.messages` mechanism on the `Session` struct

## Scenarios (6)

All tagged `@HOOK-016` in `spec/features/agent-lifecycle-hooks.feature`:
- user_prompt_submit Blocking (3): block via exit code, block via JSON, allow+inject
- session_end (1): receives reason and executes
- notification (1): fires via global engine
- Sequential Execution (1): commands run in order
