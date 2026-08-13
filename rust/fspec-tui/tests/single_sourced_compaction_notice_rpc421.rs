//! Feature: spec/features/single-sourced-compaction-notice.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! RPC-421 — the `/compact` slash handler must emit NO success notice from
//! the RPC result (whose numbers are measured before DAG injection and are
//! therefore fabricated). The `[compaction] ...` success notice is
//! single-sourced from the `StreamChunk::CompactionComplete` chunk handler
//! in `dispatch_stream_chunks.rs`, which fires for both `/compact` and
//! auto-compaction and carries the honest post-injection numbers
//! (CMPCT-038 apply-site emission). Exactly ONE notice lands per
//! compaction. The Err branch and silent no-op behaviours are unchanged.
//!
//! Written BEFORE the implementation — the "no immediate notice" and
//! "exactly one notice" tests MUST FAIL against the current Ok branch,
//! which formats the RPC result into notice #1 (red phase).
//!
//! Mirrors the spawned-task + action-bus round-trip pattern established by
//! `slash_compact_rpc047.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{CompactionResult, SessionId, StreamChunk};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Flatten every visible chunk's text in `id`'s scrollback into a single
/// String. Mirrors `slash_compact_rpc047.rs::session_scrollback_text`.
fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Count scrollback lines starting with `[compaction]` for `id`.
fn compaction_notice_count(app: &App, id: &SessionId) -> usize {
    session_scrollback_text(app, id)
        .lines()
        .filter(|l| l.starts_with("[compaction]"))
        .count()
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

/// Await every spawned backend task, then dispatch every queued action.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// The acknowledgement-shaped result the real producers now return:
/// real original_tokens, everything else the 0-valued sentinel.
fn ack_result(original: u32) -> CompactionResult {
    CompactionResult {
        compression_ratio: 0.0,
        original_tokens: original,
        compacted_tokens: 0,
        turns_summarized: 0,
        turns_kept: 0,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact Ok emits no immediate success notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_ok_emits_no_immediate_success_notice() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.0, original_tokens 8000, compacted_tokens 0, turns_summarized 0, turns_kept 0
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_ok(ack_result(8_000));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));
    wait_until(
        || mock.compact_session_calls() == 1,
        "backend.compact_session call count to reach 1",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then backend.compact_session is called exactly once with session_id s-1
    assert_eq!(mock.compact_session_calls(), 1);
    assert_eq!(mock.last_compact_session(), Some(sid("s-1")));

    // @step And s-1's scrollback contains no line starting with "[compaction]"
    assert_eq!(
        compaction_notice_count(&app, &sid("s-1")),
        0,
        "the /compact Ok branch must NOT emit a success notice from the RPC \
         result — got scrollback {:?}",
        session_scrollback_text(&app, &sid("s-1")),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Exactly one compaction notice lands when the CompactionComplete
// chunk arrives after /compact
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exactly_one_notice_when_compaction_complete_chunk_arrives_after_compact() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.0, original_tokens 8000, compacted_tokens 0, turns_summarized 0, turns_kept 0
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_ok(ack_result(8_000));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));
    wait_until(
        || mock.compact_session_calls() == 1,
        "backend.compact_session call count to reach 1",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 75.0, original_tokens 8000, compacted_tokens 2000, turns_summarized 6, turns_kept 2 is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                compression_ratio: 75.0,
                original_tokens: 8_000,
                compacted_tokens: 2_000,
                turns_summarized: 6,
                turns_kept: 2,
            },
        },
    ));
    drain_pending(&mut app).await;

    // @step Then s-1's scrollback contains exactly one line starting with "[compaction]"
    assert_eq!(
        compaction_notice_count(&app, &sid("s-1")),
        1,
        "exactly ONE compaction notice per /compact — got scrollback {:?}",
        session_scrollback_text(&app, &sid("s-1")),
    );

    // @step And that line equals "[compaction] 75.0% reduction (8000 → 2000 tokens, 6 turns summarised)"
    let text = session_scrollback_text(&app, &sid("s-1"));
    let expected = "[compaction] 75.0% reduction (8000 \u{2192} 2000 tokens, 6 turns summarised)";
    assert!(
        text.lines().any(|l| l == expected),
        "expected `{expected}` in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact failure still emits the error notice and no compaction
// notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_failure_still_emits_error_notice_and_no_compaction_notice() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Err("out of memory")
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_err("out of memory".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched and the RPC round-trip drains
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));
    wait_until(
        || mock.compact_session_calls() == 1,
        "backend.compact_session call count to reach 1",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then s-1's scrollback contains a line equal to "[error] /compact failed: out of memory"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[error] /compact failed: out of memory"),
        "expected the unchanged Err-branch notice in s-1 scrollback, got {text:?}",
    );

    // @step And s-1's scrollback contains no line starting with "[compaction]"
    assert_eq!(
        compaction_notice_count(&app, &sid("s-1")),
        0,
        "no compaction notice may accompany a failed /compact — got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-compaction emits exactly one notice without a preceding
// /compact
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compaction_emits_exactly_one_notice_without_preceding_compact() {
    // @step Given an App with an open session s-1 and no /compact command issued
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(mock.compact_session_calls(), 0);

    // @step When ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 40.0, original_tokens 5000, compacted_tokens 3000, turns_summarized 4, turns_kept 2 is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                compression_ratio: 40.0,
                original_tokens: 5_000,
                compacted_tokens: 3_000,
                turns_summarized: 4,
                turns_kept: 2,
            },
        },
    ));
    drain_pending(&mut app).await;

    // @step Then s-1's scrollback contains exactly one line starting with "[compaction]"
    assert_eq!(
        compaction_notice_count(&app, &sid("s-1")),
        1,
        "auto-compaction must land exactly one notice — got scrollback {:?}",
        session_scrollback_text(&app, &sid("s-1")),
    );
}
