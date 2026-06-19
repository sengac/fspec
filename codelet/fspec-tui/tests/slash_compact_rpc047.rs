//! RPC-047 — `/compact` slash command + compaction progress footer.
//!
//! Feature: spec/features/slash-command-compact.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Compact)`
//! so it reaches PAST the notice-fallback and into the backend's
//! `compact_session(session_id)` RPC. On success the focused session's
//! scrollback gains a `[compaction] X.X% reduction (Y → Z tokens, T
//! turns summarised)` chunk; on failure it gains a `[error] /compact
//! failed: {reason}` chunk. With no current session, /compact is a
//! silent no-op. Background sessions are never touched.
//!
//! Also exercises:
//!   - `StreamChunk::CompactionComplete` dispatcher handler clears the
//!     per-session `compaction_progress` entry and emits the same
//!     compaction notice.
//!   - `SessionFooter` renders the `[compacting: <phase> <c>/<t>]` +
//!     `▰▰▰▰▰▱▱▱▱▱` progress segment whenever the focused session has
//!     `compaction_progress = Some(...)`.
//!
//! Mirrors the spawned-task + action-bus round-trip pattern established
//! by `slash_clear_rpc046.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::agent::SessionFooter;
use codelet_fspec_tui::{Action, App, FspecBackend, RenderedChunk};
use codelet_rpc_types::{
    CompactionProgress, CompactionResult, SessionId, StreamChunk, WorkspaceInfo,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Push `count` raw scrollback chunks into the SessionContext for `id`.
/// Mirrors the `seed_chunks` helper in `slash_clear_rpc046.rs`.
#[allow(dead_code)]
fn seed_chunks(app: &mut App, id: &SessionId, count: usize) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present for seeded id");
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            seq: i as u64,
            lines: vec![Line::from(format!("seed-{i}"))],
            source: None,
        });
    }
}

/// Flatten every visible chunk's text in `id`'s scrollback into a single
/// String. Mirrors `slash_clear_rpc046.rs::session_scrollback_text`.
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

fn ok_result(ratio: f64, original: u32, compacted: u32, turns: u32) -> CompactionResult {
    CompactionResult {
        compression_ratio: ratio,
        original_tokens: original,
        compacted_tokens: compacted,
        turns_summarized: turns,
        turns_kept: 0,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact calls backend.compact_session for the focused session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_calls_backend_compact_session_for_focused_session() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.4, original_tokens 10000, compacted_tokens 4000, turns_summarized 12, turns_kept 3
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_ok(ok_result(0.4, 10_000, 4_000, 12));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(mock.compact_session_calls(), 0);

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));

    // @step Then within 1 second backend.compact_session is called exactly once with session_id s-1
    wait_until(
        || mock.compact_session_calls() == 1,
        "backend.compact_session call count to reach 1",
    )
    .await;
    assert_eq!(mock.compact_session_calls(), 1);
    assert_eq!(mock.last_compact_session(), Some(sid("s-1")));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact emits a success notice into the originating session's
// scrollback on Ok
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_emits_success_notice_into_originating_session_scrollback() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 0.4, original_tokens 10000, compacted_tokens 4000, turns_summarized 12, turns_kept 3
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_ok(ok_result(0.4, 10_000, 4_000, 12));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));

    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)"
    let text = session_scrollback_text(&app, &sid("s-1"));
    let expected = "[compaction] 60.0% reduction (10000 \u{2192} 4000 tokens, 12 turns summarised)";
    assert!(
        text.lines().any(|l| l == expected),
        "expected `{expected}` line in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact emits an error notice into the originating session's
// scrollback on Err
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_emits_error_notice_into_originating_session_scrollback() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Err("out of memory")
    let mock = Arc::new(MockBackend::new());
    mock.set_compact_session_result_err("out of memory".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));

    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /compact failed: out of memory"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[error] /compact failed: out of memory"),
        "expected `[error] /compact failed: out of memory` line in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_with_no_current_session_is_a_silent_no_op() {
    // @step Given an App with NO current session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));

    tokio::time::sleep(Duration::from_millis(100)).await;
    drain_pending(&mut app).await;

    // @step Then backend.compact_session is never called
    assert_eq!(
        mock.compact_session_calls(),
        0,
        "compact_session must NOT be called when there is no current session",
    );

    // @step And no scrollback chunk is appended to any session
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        0,
        "no sessions should exist for the no-op assertion",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /compact only affects the focused session — background
// sessions are untouched
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_only_affects_focused_session_background_untouched() {
    // @step Given an App with two open sessions s-1 (focused) and s-2 (background)
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's compact_session returns Ok with compression_ratio 0.5, original_tokens 1000, compacted_tokens 500, turns_summarized 4, turns_kept 1
    mock.set_compact_session_result_ok(ok_result(0.5, 1_000, 500, 4));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionPrev);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Compact));

    // @step Then within 1 second backend.compact_session is called exactly once with session_id s-1
    wait_until(
        || mock.compact_session_calls() == 1,
        "backend.compact_session call count to reach 1",
    )
    .await;
    assert_eq!(mock.compact_session_calls(), 1);
    assert_eq!(mock.last_compact_session(), Some(sid("s-1")));

    drain_pending(&mut app).await;

    // @step And within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 50.0% reduction (1000 → 500 tokens, 4 turns summarised)"
    let s1_text = session_scrollback_text(&app, &sid("s-1"));
    let expected = "[compaction] 50.0% reduction (1000 \u{2192} 500 tokens, 4 turns summarised)";
    assert!(
        s1_text.lines().any(|l| l == expected),
        "expected `{expected}` in s-1 scrollback, got {s1_text:?}",
    );

    // @step And s-2's scrollback does NOT contain any chunk whose text starts with "[compaction]"
    let s2_text = session_scrollback_text(&app, &sid("s-2"));
    assert!(
        s2_text.lines().all(|l| !l.starts_with("[compaction]")),
        "s-2 scrollback must NOT contain a `[compaction]` line, got {s2_text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionFooter renders the compaction progress segment when
// progress is Some
// ─────────────────────────────────────────────────────────────────────────

fn row_text(buf: &Buffer, y: u16) -> String {
    let mut s = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

#[test]
fn session_footer_renders_compaction_progress_when_some() {
    // @step Given an App with an open session s-1 whose compaction_progress is Some with phase "summarising messages", current 5, total 10
    let progress = CompactionProgress {
        phase: "summarising messages".to_string(),
        current: 5,
        total: 10,
    };
    let workspace = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    };

    // @step When SessionFooter is rendered into an 80x1 buffer
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 1));
    SessionFooter {
        workspace: Some(&workspace),
        compaction_progress: Some(&progress),
        supervisor_pending_count: 0,
    }
    .render(buf.area, &mut buf);

    let row = row_text(&buf, 0);

    // @step Then the rendered row contains the substring "[compacting: summarising messages 5/10]"
    assert!(
        row.contains("[compacting: summarising messages 5/10]"),
        "expected `[compacting: summarising messages 5/10]` substring in row, got {row:?}",
    );
    // @step And the rendered row contains the substring "▰▰▰▰▰▱▱▱▱▱"
    assert!(
        row.contains(
            "\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B1}\u{25B1}\u{25B1}\u{25B1}\u{25B1}"
        ),
        "expected `▰▰▰▰▰▱▱▱▱▱` bar in row, got {row:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionFooter omits the compaction segment when progress is None
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn session_footer_omits_compaction_segment_when_none() {
    // @step Given an App with an open session s-1 whose compaction_progress is None
    let workspace = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    };

    // @step When SessionFooter is rendered into an 80x1 buffer
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 1));
    SessionFooter {
        workspace: Some(&workspace),
        compaction_progress: None,
        supervisor_pending_count: 0,
    }
    .render(buf.area, &mut buf);

    let row = row_text(&buf, 0);

    // @step Then the rendered row does NOT contain the substring "[compacting:"
    assert!(
        !row.contains("[compacting:"),
        "compaction segment must be absent when progress is None, got {row:?}",
    );
    // @step And the rendered row does NOT contain the substring "▰"
    assert!(
        !row.contains('\u{25B0}'),
        "▰ glyph must be absent when progress is None, got {row:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CompactionComplete chunk clears progress and emits a
// completion notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compaction_complete_chunk_clears_progress_and_emits_notice() {
    // @step Given an App with an open session s-1 whose compaction_progress is Some with phase "summarising", current 3, total 10
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut().set_compaction_progress(
        sid("s-1"),
        CompactionProgress {
            phase: "summarising".to_string(),
            current: 3,
            total: 10,
        },
    );
    assert!(app
        .agent_view_store()
        .compaction_progress_for(&sid("s-1"))
        .is_some());

    // @step When ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 0.25, original_tokens 8000, compacted_tokens 2000, turns_summarized 6, turns_kept 2 is dispatched
    let result = CompactionResult {
        compression_ratio: 0.25,
        original_tokens: 8_000,
        compacted_tokens: 2_000,
        turns_summarized: 6,
        turns_kept: 2,
    };
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: result,
        },
    ));

    drain_pending(&mut app).await;

    // @step Then compaction_progress_for(s-1) becomes None
    assert!(
        app.agent_view_store()
            .compaction_progress_for(&sid("s-1"))
            .is_none(),
        "compaction_progress for s-1 must be cleared after CompactionComplete",
    );

    // @step And within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 75.0% reduction (8000 → 2000 tokens, 6 turns summarised)"
    let text = session_scrollback_text(&app, &sid("s-1"));
    let expected = "[compaction] 75.0% reduction (8000 \u{2192} 2000 tokens, 6 turns summarised)";
    assert!(
        text.lines().any(|l| l == expected),
        "expected `{expected}` in s-1 scrollback, got {text:?}",
    );
}
