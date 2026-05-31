# RPC-071 — Test Plan

> Comprehensive test coverage required to prove the fix lands and prevent
> regression.

---

## 1. Unit Tests — `session_context.rs` (in-file `#[cfg(test)] mod tests`)

Add the following new tests alongside the existing
`new_context_has_empty_scrollback_and_draft`,
`record_chunk_appends_and_bumps_seq`, and
`reset_scrollback_drops_chunks_and_resets_seq` tests.

### 1.1 `user_input_chunk_renders_as_user_arrow_text`

```rust
#[test]
fn user_input_chunk_renders_as_user_arrow_text() {
    // @step Given a fresh SessionContext for session s-1
    let mut ctx = SessionContext::new(SessionId::new("s-1"));

    // @step When record_chunk receives StreamChunk::UserInput { text: "hello" }
    ctx.record_chunk(&StreamChunk::user_input("hello".to_string()));

    // @step Then the scrollback contains exactly one entry
    assert_eq!(ctx.scrollback.chunk_count(), 1);

    // @step And the entry's rendered text equals "user> hello"
    let rendered = render_first(&ctx);
    assert_eq!(rendered, "user> hello");

    // @step And the seq cursor has advanced to 1
    assert_eq!(ctx.scrollback_next_seq, 1);
}
```

### 1.2 `session_state_change_chunks_do_not_appear_in_scrollback`

```rust
#[test]
fn session_state_change_chunks_do_not_appear_in_scrollback() {
    // @step Given a fresh SessionContext
    let mut ctx = SessionContext::new(SessionId::new("s-1"));

    // @step When record_chunk receives StreamChunk::SessionStateChange { state: Running }
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Running));

    // @step And record_chunk receives StreamChunk::SessionStateChange { state: Idle }
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Idle));

    // @step Then the scrollback remains empty
    assert_eq!(ctx.scrollback.chunk_count(), 0);

    // @step And the seq cursor has NOT advanced
    assert_eq!(ctx.scrollback_next_seq, 0);
}
```

### 1.3 `all_state_only_chunks_are_suppressed`

```rust
#[test]
fn all_state_only_chunks_are_suppressed() {
    let mut ctx = SessionContext::new(SessionId::new("s-1"));

    let state_only: Vec<StreamChunk> = vec![
        StreamChunk::session_state_change(SessionState::Running),
        StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc".to_string()),
        },
        StreamChunk::DebugStateChange { enabled: true },
        StreamChunk::FooterStateUpdate {
            cwd: "/tmp".to_string(),
            display_path: "/tmp".to_string(),
            is_git_repo: false,
            branch: None,
        },
        StreamChunk::FspecCommandRequest {
            fspec_request: FspecRequest::default(),
        },
        StreamChunk::FspecCommandResult {
            fspec_result: FspecResult::default(),
        },
        StreamChunk::WorkUnitsUpdate { work_units: vec![] },
        StreamChunk::SupervisorPendingInjection {
            supervisor_pending_injection: SupervisorPendingInjectionInfo::default(),
        },
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult::default(),
        },
        StreamChunk::TokenUpdate {
            tokens: TokenTracker::default(),
        },
        StreamChunk::ContextFillUpdate {
            context_fill: ContextFillInfo::default(),
        },
        StreamChunk::ToolCall {
            tool_call: ToolCallInfo::default(),
            correlation_id: None,
            observed_correlation_ids: None,
        },
        StreamChunk::ToolResult {
            tool_result: ToolResultInfo::default(),
            correlation_id: None,
            observed_correlation_ids: None,
        },
        StreamChunk::ToolProgress {
            tool_progress: ToolProgressInfo::default(),
            correlation_id: None,
            observed_correlation_ids: None,
        },
    ];

    for c in &state_only {
        ctx.record_chunk(c);
    }

    assert_eq!(ctx.scrollback.chunk_count(), 0,
        "state-only chunks must not produce scrollback lines");
    assert_eq!(ctx.scrollback_next_seq, 0,
        "suppressed chunks must not bump the seq cursor");
}
```

### 1.4 `interleaved_visible_and_silent_chunks_keep_seq_monotonic`

```rust
#[test]
fn interleaved_visible_and_silent_chunks_keep_seq_monotonic() {
    let mut ctx = SessionContext::new(SessionId::new("s-1"));

    ctx.record_chunk(&StreamChunk::user_input("hi".to_string()));
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Running));
    ctx.record_chunk(&StreamChunk::text("hello back".to_string()));
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Idle));
    ctx.record_chunk(&StreamChunk::Done);

    assert_eq!(ctx.scrollback.chunk_count(), 3); // user, text, done
    assert_eq!(ctx.scrollback_next_seq, 3);
}
```

### 1.5 `screenshot_reproduction_renders_only_user_input`

This is the literal screenshot reproduction. The single most important test.

```rust
#[test]
fn screenshot_reproduction_renders_only_user_input() {
    // Reproduces the chunk sequence from
    // /Users/rquast/Desktop/Screenshot 2026-05-27 at 8.46.38 am.png
    let mut ctx = SessionContext::new(SessionId::new("s-1"));

    ctx.record_chunk(&StreamChunk::user_input("please review this card".to_string()));
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Running));
    ctx.record_chunk(&StreamChunk::session_state_change(SessionState::Idle));

    let lines = render_all(&ctx);
    assert_eq!(lines, vec!["user> please review this card".to_string()],
        "scrollback must contain ONLY the user input line (regression: \
         pre-RPC-071 also rendered raw Debug output for the two state chunks)");
}
```

### 1.6 Helper

```rust
fn render_first(ctx: &SessionContext) -> String {
    render_all(ctx).into_iter().next().unwrap_or_default()
}

fn render_all(ctx: &SessionContext) -> Vec<String> {
    ctx.scrollback
        .iter()
        .flat_map(|c| &c.lines)
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect()
}
```

---

## 2. Integration Test — `codelet/fspec-tui/tests/chunk_rendering_parity_rpc071.rs`

New file. Drives the full `App::dispatch` path the production binary uses
(not just the bare `SessionContext`), so we prove the fix lands at the
right level.

```rust
//! Feature: spec/features/agentview-chunk-rendering.feature
//!
//! Regression guard for RPC-071. Reproduces the exact chunk sequence
//! seen in the 2026-05-27 screenshot and asserts the rendered scrollback
//! contains only the `user>` line.

use codelet_rpc_types::{SessionId, SessionState, StreamChunk};
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::test_helpers::AppHarness;

mod common;

#[tokio::test]
async fn screenshot_reproduction_no_debug_output_in_scrollback() {
    // @step Given a running App with an attached AgentView for session s-1
    let mut harness = AppHarness::new_with_session("s-1").await;

    // @step When the chunk stream emits UserInput, SessionStateChange(Running), SessionStateChange(Idle)
    let session = SessionId::new("s-1");
    harness.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::user_input("please review this card".to_string()),
    )).await;
    harness.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::session_state_change(SessionState::Running),
    )).await;
    harness.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::session_state_change(SessionState::Idle),
    )).await;

    // @step Then the rendered AgentView scrollback contains exactly one line
    let scrollback = harness.rendered_scrollback("s-1");
    assert_eq!(scrollback.len(), 1,
        "expected 1 scrollback line, got {:?}", scrollback);

    // @step And that line equals "user> please review this card"
    assert_eq!(scrollback[0], "user> please review this card");

    // @step And the session's status pill reflects Idle (state was still consumed)
    assert_eq!(
        harness.agent_view_store().session_status(&session),
        codelet_rpc_types::SessionStatus::Idle,
    );

    // @step And no rendered line contains the literal substring "SessionStateChange"
    for line in &scrollback {
        assert!(!line.contains("SessionStateChange"),
            "scrollback leaked Debug output: {line}");
        assert!(!line.contains("UserInput {"),
            "scrollback leaked Debug output: {line}");
    }
}
```

---

## 3. Exhaustiveness / Future-Proofing Test

`codelet/fspec-tui/tests/chunk_rendering_exhaustive_rpc071.rs` — a
compile-time assertion that the match arm is exhaustive. Because the new
`chunk_to_lines` has no catch-all, this is **automatically enforced** by
the Rust compiler: any future addition to `StreamChunk` in
`codelet-rpc-types` will fail to compile until `chunk_to_lines` is updated.

No runtime test needed. The fact that the match arm is exhaustive is the
test.

We add a doc comment + a `// SAFETY: exhaustive match — adding a new
StreamChunk variant in codelet-rpc-types will break this file. See
spec/attachments/RPC-071/fix-plan.md.` annotation above the match.

---

## 4. Validation Steps

```bash
# 1. Unit tests
cd codelet
cargo test -p codelet-fspec-tui --lib --
   session_context::tests::user_input_chunk_renders_as_user_arrow_text
cargo test -p codelet-fspec-tui --lib --
   session_context::tests::screenshot_reproduction_renders_only_user_input
cargo test -p codelet-fspec-tui --lib --
   session_context::tests::all_state_only_chunks_are_suppressed

# 2. Integration test
cargo test -p codelet-fspec-tui --test chunk_rendering_parity_rpc071

# 3. Full workspace (must still pass)
cargo test --workspace

# 4. Manual binary smoke test
cargo run -p codelet-fspec --bin fspec
#   → Press Enter on a DONE work unit
#   → Type "please review this card", Enter
#   → Scrollback contains ONE line:  user> please review this card
```

---

## 5. Test Coverage Linking (post-implementation)

```
fspec link-coverage agentview-chunk-rendering \
  --scenario "UserInput chunk renders as user> prefix" \
  --testFile codelet/fspec-tui/src/store/agent_view/session_context.rs \
  --testLines <range> \
  --implFile codelet/fspec-tui/src/store/agent_view/session_context.rs \
  --implLines <range>

fspec link-coverage agentview-chunk-rendering \
  --scenario "Screenshot reproduction renders only user input" \
  --testFile codelet/fspec-tui/tests/chunk_rendering_parity_rpc071.rs \
  --testLines <range> \
  --implFile codelet/fspec-tui/src/store/agent_view/session_context.rs \
  --implLines <range>
```
