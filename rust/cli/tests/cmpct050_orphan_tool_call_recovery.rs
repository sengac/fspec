#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/orphan-tool-call-defeats-execute-compaction-trailing-user-pop-removes-a-toolresult-after-the-cmpct-029-cleanup-ran.feature
//!
//! CMPCT-050: Orphan tool_call defeats execute_compaction.
//!
//! Regression: on Path C the CMPCT-029 cleanup (reconcile + drain +
//! inject_synthetic) ran BEFORE `begin_compaction_recovery`, whose
//! trailing-User pop (`matches!(last, Message::User { .. })`) removed a
//! `User(ToolResult)` and re-orphaned its tool_call — so
//! `execute_compaction`'s defensive guard refused and the user saw
//! "Compaction failed: execute_compaction refuses to proceed: 1 orphan
//! tool_call(s)... - will retry on next turn".
//!
//! Fix contract pinned here:
//! 1. The pop must NOT remove a trailing `User(ToolResult)` (mid-turn
//!    conversation state) — plain text prompts are still popped.
//! 2. The orphan safety net (synthetic cancelled tool_results) must run at
//!    the end of `begin_compaction_recovery` — AFTER the pop and the
//!    partial-text flush — so EVERY entry path (B/C/D/032/overflow) leaves
//!    `session.messages` tool-pair-clean before `execute_compaction`.
//! 3. The manual `/compact` twins (sessions `compact_session`, NAPI
//!    `session_compact`) must sanitize before `execute_compaction` so a
//!    persisted orphan cannot wedge the session.

#[path = "support/offline_session.rs"]
mod offline_session;
#[path = "support/stream_loop_source.rs"]
mod stream_loop_source_tree;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use codelet_cli::interactive::begin_compaction_recovery;
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive_helpers::{
    execute_compaction, inject_synthetic_tool_results_for_orphans, validate_no_orphan_tool_calls,
};
use codelet_cli::session::Session;
use codelet_core::{StreamingTokenDisplay, TokenState};
use rig::message::{AssistantContent, Message, ToolCall, ToolFunction, UserContent};
use rig::OneOrMany;

// ============================================================================
// Shared fixtures
// ============================================================================

/// Fake StreamOutput that records every emitted event.
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn count<F: Fn(&StreamEvent) -> bool>(&self, pred: F) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Process-wide tempdir kept alive for the lifetime of the test binary —
/// `execute_compaction` fans out into debug-capture which requires
/// `set_data_directory` (same pattern as compaction_tool_call_preservation_test).
static TEST_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

fn ensure_test_data_dir() {
    TEST_DATA_DIR.get_or_init(|| {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        codelet_test_helpers::isolated_user_directory()
            .expect("BUG-640: isolate the user directory");
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("failed to set data directory");
        temp_dir
    });
}

fn fresh_session() -> Session {
    offline_session::session().expect("failed to create test session")
}

fn fresh_token_state() -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }))
}

fn make_tool_call(id: &str, call_id: Option<&str>, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_id: call_id.map(ToString::to_string),
        function: ToolFunction {
            name: name.to_string(),
            arguments: serde_json::json!({}),
        },
        signature: None,
        additional_params: None,
    }
}

fn assistant_tool_call_message(tc: ToolCall) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::ToolCall(tc)),
    }
}

fn user_tool_result_message(id: &str, call_id: Option<&str>, body: &str) -> Message {
    let content = OneOrMany::one(rig::message::ToolResultContent::Text(rig::message::Text {
        text: body.to_string(),
    }));
    let user_content = match call_id {
        Some(cid) => UserContent::tool_result_with_call_id(id, cid.to_string(), content),
        None => UserContent::tool_result(id, content),
    };
    Message::User {
        content: OneOrMany::one(user_content),
    }
}

fn user_text_message(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    }
}

fn has_tool_result(msg: &Message) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content
                .iter()
                .any(|c| matches!(c, UserContent::ToolResult(_)))
    )
}

fn is_tool_result_with_call_id(msg: &Message, call_id: &str) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content.iter().any(|c| matches!(
                c,
                UserContent::ToolResult(tr) if tr.call_id.as_deref() == Some(call_id)
            ))
    )
}

fn is_plain_user_text(msg: &Message, text: &str) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content.iter().any(|c| matches!(
                c,
                UserContent::Text(t) if t.text == text
            ))
    )
}

/// Read a workspace-relative source file. Tests run with the crate dir as
/// CWD, so walk up from `CARGO_MANIFEST_DIR` (rust/cli) two levels to the
/// repo root — same pattern as `tool_progress_registration_key_rpc398.rs`.
fn read_repo_file(relative: &str) -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate dir (rust/cli) must have two parents (repo root)");
    let path = repo_root.join(relative);
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Line number (1-based) of the first line containing `needle`.
fn line_number_of(src: &str, needle: &str) -> Option<usize> {
    src.lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map(|(idx, _)| idx + 1)
}

/// Extract the body of `fn_name` (from its definition line to the next
/// top-level item, or EOF), for ordering assertions.
fn function_body(src: &str, fn_name: &str) -> String {
    let start =
        line_number_of(src, &format!("pub fn {fn_name}")).expect("function {fn_name} must exist");
    let lines: Vec<&str> = src.lines().collect();
    let body_start = start - 1; // 0-based
    let end_rel = lines
        .iter()
        .skip(body_start + 1)
        .position(|l| l.starts_with("pub ") || l.starts_with("pub("))
        .unwrap_or(lines.len() - body_start);
    lines[body_start..body_start + end_rel].join("\n")
}

// ============================================================================
// Scenario 1: begin_compaction_recovery preserves a trailing
// User(ToolResult) at the tail of session.messages
// ============================================================================

// ============================================================================
// Scenario 2 (control): A trailing plain User text prompt is still popped
// (no behavior change)
// ============================================================================

// ============================================================================
// Scenario 3: PromptCancelled firing right after a tool result lets
// compaction resume without a failure notification
// ============================================================================

// ============================================================================
// Scenario 4: Orphan safety net closes orphans after all mutations on
// every entry path
// ============================================================================

// ============================================================================
// Scenario 5: Manual /compact recovers a persisted orphan instead of failing
// ============================================================================

// ============================================================================
// Scenario 6: execute_compaction still refuses when an orphan survives
// with no recovery performed (guard unchanged)
// ============================================================================

#[path = "cmpct050_orphan_tool_call_recovery/all_in_loop_entry.rs"]
mod all_in_loop_entry;
#[path = "cmpct050_orphan_tool_call_recovery/begin_compaction_recovery_preserves.rs"]
mod begin_compaction_recovery_preserves;
