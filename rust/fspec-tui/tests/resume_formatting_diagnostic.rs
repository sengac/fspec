//! Diagnostic: reproduce the resume flow and check rendered output.
//!
//! The screenshot shows text broken into extremely narrow lines after resume.
//! This test reproduces the exact resume path and dumps the buffer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::{MockBackend, buffer_to_rows, render_one_frame, test_app};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
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

// ─────────────────────────────────────────────────────────────────────────
// Reproduce the EXACT resume path: AttachToSession → resume_session →
// SessionResumeComplete → get_buffered_output → ChunkReceived × N
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_flow_renders_text_correctly() {
    // @step Given an App with session s-1 in AgentView mode
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));

    // Seed the mock's buffered_output so get_buffered_output returns chunks
    let resume_sid = SessionId::new("s-resume");
    mock.push_session_created(resume_sid.clone());
    let chunks = vec![
        StreamChunk::user_input("what is this card about?".to_string()),
        StreamChunk::text("This is a test response with multiple words on one line".to_string()),
        StreamChunk::Done,
    ];
    mock.set_buffered_output(chunks);

    // @step When I dispatch AttachToSession for s-resume
    app.dispatch(Action::AttachToSession(resume_sid.clone()));

    // @step And I drain all pending tasks (resume_session + get_buffered_output)
    let result = timeout(Duration::from_secs(2), async {
        drain_pending(&mut app).await;
    })
    .await;
    assert!(result.is_ok(), "drain_pending should not timeout");

    // @step When I render the App into an 80x24 buffer
    let (mut app, mut terminal) = {
        let backend: Arc<dyn FspecBackend> = mock.clone();
        test_app(backend)
    };
    // Re-attach to the app we built above — we need to use the same app state.
    // Actually, let's just render directly.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new");
    let buf = render_one_frame(&mut terminal, &mut app);

    let rows = buffer_to_rows(&buf);
    eprintln!("=== resume_flow_render ===");
    for (i, row) in rows.iter().enumerate() {
        eprintln!("y={:2}: {}", i, row);
    }

    // @step Then the user line appears intact on one row
    let mut found_user = false;
    for (y, row) in rows.iter().enumerate() {
        if row.contains("You: what is this card about?") {
            found_user = true;
            eprintln!("✓ Found user line at y={}: {:?}", y, row);
            break;
        }
    }
    assert!(found_user, "User line should be intact");

    // @step And the assistant line appears intact on one row
    let mut found_assistant = false;
    for (y, row) in rows.iter().enumerate() {
        if row.contains("This is a test response") {
            found_assistant = true;
            eprintln!("✓ Found assistant line at y={}: {:?}", y, row);
            break;
        }
    }
    assert!(found_assistant, "Assistant line should be intact");
}

// ─────────────────────────────────────────────────────────────────────────
// Check the chunk lines directly after resume
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_flow_chunk_line_widths() {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));

    let resume_sid = SessionId::new("s-resume");
    mock.push_session_created(resume_sid.clone());
    let chunks = vec![
        StreamChunk::user_input("All tests pass including the 3 resume-specific tests".to_string()),
        StreamChunk::Done,
    ];
    mock.set_buffered_output(chunks);

    app.dispatch(Action::AttachToSession(resume_sid.clone()));

    let result = timeout(Duration::from_secs(2), async {
        drain_pending(&mut app).await;
    })
    .await;
    assert!(result.is_ok());

    // Check the chunk lines directly
    let ctx = app
        .agent_view_store()
        .session_context_for(&resume_sid)
        .expect("session context for resumed session");

    let chunks = ctx.scrollback.chunks();
    eprintln!("chunk_count = {}", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        eprintln!("chunk[{}] seq={} lines={}", i, chunk.seq, chunk.lines.len());
        for (j, line) in chunk.lines.iter().enumerate() {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            let width = text.chars().count();
            eprintln!("  line[{}] width={} content={:?}", j, width, text);
        }
    }

    // Every non-empty line should be wider than 5 chars
    for chunk in chunks {
        for line in &chunk.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if !text.trim().is_empty() {
                assert!(
                    text.chars().count() > 5,
                    "Line too narrow: content={:?}",
                    text
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Now render AFTER resume and check the buffer for broken wrapping
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_flow_render_after_drain() {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));

    let resume_sid = SessionId::new("s-resume");
    mock.push_session_created(resume_sid.clone());
    let chunks = vec![
        StreamChunk::user_input("All tests pass including the 3 resume-specific tests".to_string()),
        StreamChunk::text("This is a long assistant response that should fit on one line in an 80-column terminal".to_string()),
        StreamChunk::Done,
    ];
    mock.set_buffered_output(chunks);

    app.dispatch(Action::AttachToSession(resume_sid.clone()));

    let result = timeout(Duration::from_secs(2), async {
        drain_pending(&mut app).await;
    })
    .await;
    assert!(result.is_ok());

    // Render and dump
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new");
    let buf = render_one_frame(&mut terminal, &mut app);

    let rows = buffer_to_rows(&buf);
    eprintln!("=== resume_flow_render_after_drain ===");
    for (i, row) in rows.iter().enumerate() {
        eprintln!("y={:2}: {}", i, row);
    }

    // Check that the user line is intact
    let mut found_user = false;
    for (y, row) in rows.iter().enumerate() {
        if row.contains("You: All tests pass including") {
            found_user = true;
            eprintln!("✓ Found user line at y={}: {:?}", y, row);
            break;
        }
    }
    assert!(found_user, "User line should be intact on one row");

    // Check that the assistant line is intact
    let mut found_assistant = false;
    for (y, row) in rows.iter().enumerate() {
        if row.contains("This is a long assistant response") {
            found_assistant = true;
            eprintln!("✓ Found assistant line at y={}: {:?}", y, row);
            break;
        }
    }
    assert!(found_assistant, "Assistant line should be intact on one row");
}
