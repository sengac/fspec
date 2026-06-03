# RPC-090: Lifecycle Hooks — AST Research (Source-Shape Sweep)

## Scope

Regression-shape coverage card. Mirrors the RPC-089 pattern: pin the
canonical NAPI loop's lifecycle-hook invocation points so the four
HOOK-013 call sites (`session_start`, `user_prompt_submit`,
`post_tool_use`, `session_end`) can never silently regress to the
RPC-072 stub state where none of them fired.

Implementation already exists. This card creates the regression-shape
tests + feature file that pin the call-site structure in source.

## Canonical structures

### `codelet/agent-loop/src/agent_loop.rs`

**Imports (line 30-32):**
```rust
use codelet_core::lifecycle_hooks::{
    run_session_end, run_session_start, run_user_prompt, HookMessageLevel,
};
```

The three loop-side hook runners are imported by name in a single
brace-grouped use statement. `run_post_tool` is NOT here — it is imported
by `background_output.rs` because that's where post-tool firing lives.

**session_start invocation (line 89-112)** — fires before the loop:
```rust
// HOOK-013: Fire session_start hooks
if let Some(ref hooks) = session.lifecycle_hooks {
    let ctx = session.hook_context();
    let outcome = run_session_start(hooks, &ctx, "startup").await;
    if !outcome.additional_context.is_empty() {
        let mut inner = session.inner.lock().await;
        let combined_context = outcome.additional_context.join("\n");
        inner.add_system_reminder(
            codelet_cli::session::SystemReminderType::FspecWorkflow,
            &combined_context,
        );
        drop(inner);
    }
    for msg in &outcome.messages {
        if msg.level == HookMessageLevel::Warning || msg.level == HookMessageLevel::Error {
            tracing::warn!("[HOOK-013] session_start hook: {}", msg.content);
            session.handle_output(StreamChunk::user_notification(
                format!("Hook: {}", msg.content),
                NotificationSeverity::Warning,
            ));
        }
    }
}
```

**Invariants:**
- `run_session_start` is called with the literal phase string `"startup"`
- It fires BEFORE the main `loop {` block opens
- `outcome.additional_context` is joined and pushed via `add_system_reminder` with `SystemReminderType::FspecWorkflow`
- Warning/Error messages emit `StreamChunk::user_notification` with `NotificationSeverity::Warning`

**session_end invocation (line 146-149)** — fires on `input_rx` close:
```rust
result = input_rx.recv() => {
    match result {
        Some(prompt_input) => Some(InputWithImages { ... }),
        None => {
            drop(supervisor_rx);
            // HOOK-013: Fire session_end hooks before exiting
            if let Some(ref hooks) = session.lifecycle_hooks {
                let ctx = session.hook_context();
                let _outcome = run_session_end(hooks, &ctx, "exit").await;
            }
            break;
        }
    }
}
```

**Invariants:**
- `run_session_end` is called with the literal phase string `"exit"`
- The call lives INSIDE the `None =>` arm of the `input_rx.recv()` match
- It is followed by `break;` — the only way out of the agent loop
- `drop(supervisor_rx)` precedes it (lock release before hook)

**user_prompt_submit invocation (line 264-297)** — fires per turn pre-LLM:
```rust
// HOOK-013: Run user_prompt_submit hooks (can block the prompt)
if let Some(ref hooks) = session.lifecycle_hooks {
    if !hooks.user_prompt_submit.is_empty() {
        let ctx = session.hook_context();
        let outcome = run_user_prompt(hooks, &ctx, input).await;
        for msg in &outcome.messages {
            if msg.level == HookMessageLevel::Warning || msg.level == HookMessageLevel::Error {
                tracing::warn!("[HOOK-013] user_prompt_submit hook: {}", msg.content);
            }
        }
        if !outcome.allow_prompt {
            let reason = outcome.block_reason.unwrap_or_else(|| "Blocked by hook".to_string());
            tracing::warn!("[HOOK-013] Prompt blocked: {}", reason);
            session.handle_output(StreamChunk::user_notification(
                format!("Prompt blocked: {}", reason),
                NotificationSeverity::Warning,
            ));
            session.set_status(SessionStatus::Idle);
            session.handle_output(StreamChunk::done());
            continue;
        }
        if !outcome.additional_context.is_empty() {
            let mut inner_session = session.inner.lock().await;
            let combined_context = outcome.additional_context.join("\n");
            inner_session.add_system_reminder(
                codelet_cli::session::SystemReminderType::FspecWorkflow,
                &combined_context,
            );
            drop(inner_session);
        }
    }
}
```

**Invariants:**
- Guarded by `!hooks.user_prompt_submit.is_empty()` (avoid hook_context cost when no hooks configured)
- `run_user_prompt` is called with `input` (the user's text)
- Block path: `!outcome.allow_prompt` → emit `user_notification` → `set_status(Idle)` → emit `done()` → `continue;`
- The `continue;` is essential — without it, a blocked prompt would still proceed to the LLM
- Context injection mirrors session_start path

### `codelet/agent-loop/src/background_output.rs`

**Import (line 22):**
```rust
use codelet_core::lifecycle_hooks::{run_post_tool, HookMessageLevel};
```

`run_post_tool` is imported only by background_output.rs — it fires
inside the `StreamEvent::ToolResult` arm, not the loop body.

**post_tool_use invocation (line 155-206)**:
```rust
// HOOK-013: Run post_tool_use hooks (fire-and-forget with context injection)
if let Some(ref hooks) = self.session.lifecycle_hooks {
    if !hooks.post_tool_use.is_empty() {
        let tool_name_for_hook = self
            .last_tool_call
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|(name, _)| name.clone()));

        if let Some(tool_name) = tool_name_for_hook {
            let hooks_clone = hooks.clone();
            let ctx = self.session.hook_context();
            let tool_input = self.last_tool_call.lock().ok()
                .and_then(|guard| guard.as_ref().map(|(_, input)| input.clone()))
                .unwrap_or(serde_json::Value::Null);
            let tool_response = tr.content.clone();
            let session_for_hook = self.session.clone();

            tokio::spawn(async move {
                let outcome = run_post_tool(
                    &hooks_clone, &ctx, &tool_name, &tool_input, &tool_response,
                ).await;
                for context_text in &outcome.additional_context {
                    session_for_hook.handle_output(
                        StreamChunk::user_notification(
                            format!("Hook context: {}", context_text),
                            NotificationSeverity::Info,
                        ),
                    );
                }
                for msg in &outcome.messages {
                    if msg.level == HookMessageLevel::Warning
                        || msg.level == HookMessageLevel::Error
                    {
                        tracing::warn!(
                            "[HOOK-013] post_tool_use hook: {}",
                            msg.content
                        );
                    }
                }
            });
        }
    }
}
```

**Invariants:**
- Lives inside `StreamEvent::ToolResult(ref tr) =>` arm
- Guarded by `!hooks.post_tool_use.is_empty()`
- `tokio::spawn(async move { ... })` — fire-and-forget (does not block the stream)
- `run_post_tool` takes 5 args: hooks, ctx, tool_name, tool_input, tool_response
- Context lines emit with `NotificationSeverity::Info` (NOT Warning, unlike the loop-side hooks)
- Uses `self.last_tool_call` mutex captured during `StreamEvent::ToolCall` arm (line 129-132)

### `codelet/agent-loop/src/background_output.rs:129-132` — last_tool_call capture
```rust
// HOOK-013: Capture tool call info for post_tool_use hooks
if let Ok(mut last) = self.last_tool_call.lock() {
    *last = Some((tc.name.clone(), input_value));
}
```

`last_tool_call: Mutex<Option<(String, serde_json::Value)>>` field (declared
line 46) is the bridge — `ToolCall` arm writes it, `ToolResult` arm reads it
inside the spawned post_tool_use task.

## Source-string assertions (no runtime hooks needed)

The card pins the canonical structure via byte-level grep over the two
source files. No `LifecycleHooks` instance needs to be constructed — we
are not running the hooks, we are pinning that the invocation sites
remain wired correctly.
