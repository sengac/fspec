# RPC-048 — `/thinking off|low|med|high` inline-arg parsing

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 1 pt · **Depends on:** RPC-047

## Goal

Extend `codelet/fspec-tui/src/app/slash_parser.rs` to parse `/thinking <level>` arguments. Bare `/thinking` already opens the `ThinkingLevelDialog` (RPC-022). With an arg, the parser calls `backend.set_thinking_level(session_id, level)` directly without showing the dialog.

## TS reference

`AgentView.tsx` line 2824: bare `/thinking` → `setShowThinkingLevelDialog(true)`; otherwise parse `off|low|med|medium|high` and call `getRustStateSource().setBaseThinkingLevel(currentSessionId, level)`.

## Work

In `slash_parser.rs::parse_slash_command`:

```rust
if let Some(rest) = trimmed.strip_prefix("/thinking ") {
    let level = match rest.trim().to_ascii_lowercase().as_str() {
        "off" => ThinkingLevel::Off,
        "low" => ThinkingLevel::Low,
        "med" | "medium" => ThinkingLevel::Medium,
        "high" => ThinkingLevel::High,
        other => return ParsedSlash::Invalid(format!("unknown thinking level: {other}")),
    };
    return ParsedSlash::SetThinkingLevel(level);
}
```

`ParsedSlash` enum gains a `SetThinkingLevel(ThinkingLevel)` variant. The submit-line handler (in `dispatch_rpc020.rs::handle_input_submitted` around line 130-148) dispatches it to `backend.set_thinking_level(session_id, level)`.

## Acceptance criteria

1. `/thinking off`, `/thinking low`, `/thinking med`, `/thinking medium`, `/thinking high` all set the level without showing the dialog.
2. `/thinking` (bare) opens the dialog.
3. `/thinking gibberish` emits `[error] unknown thinking level: gibberish`.
4. After calling `set_thinking_level`, the `agent_view_store.thinking_level_by_session` map is updated (or the dispatcher refreshes from the backend).
5. Unit test in `codelet/fspec-tui/src/app/__tests__/slash_parser.test.rs` covers all 6 paths.

## Out of scope

- Default thinking level per-user → covered by `set_thinking_level_default` trait method (already in RPC-037).
