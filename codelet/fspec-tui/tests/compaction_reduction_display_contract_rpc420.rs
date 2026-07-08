//! RPC-420 — Compaction reduction display contract (RED phase).
//!
//! Feature: spec/features/compaction-reduction-display-contract.feature
//!
//! The wire contract: `rpc_types::CompactionResult.compression_ratio` is
//! the PERCENT of tokens removed, range [0,100] — every real producer
//! ships `compression_ratio(orig, compacted) * 100.0`. These tests fail
//! against the pre-fix TUI code, which wrongly re-derives the reduction
//! as `(1.0 - ratio) * 100.0` (a fraction-remaining convention that
//! matches no producer), turning a 99.0% reduction into `-9800` whose
//! sign is masked by `.abs()` → the infamous `COMPACTED 9800%` badge.
//!
//! Each `#[test]` maps 1:1 to a Scenario in the feature file and reuses
//! the fixture pattern of
//! `agentview_session_header_compaction_percentage_rpc100.rs` plus the
//! dual-transport harness of `rpc037_cross_transport_parity.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{CompactionResult, ContextFillInfo, SessionId, SessionStatus, StreamChunk};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tempfile::TempDir;

mod common;
use common::MockBackend;

// ───────────────────────── helpers ────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn context_fill(pct: u32) -> ContextFillInfo {
    ContextFillInfo {
        fill_percentage: pct,
        effective_tokens: 0.0,
        threshold: 0.0,
        context_window: 0.0,
    }
}

fn agent_app_with_single_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut().focus_session_index(0);
    (app, mock)
}

/// Drain every pending Action queued onto `action_tx` (e.g. the
/// `EmitSessionNotice` produced by the `CompactionComplete` handler)
/// and re-dispatch them so side-effects land before the next render.
fn drain_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Render the full App into a 100x24 `TestBackend` and return the buffer.
fn render_app_buffer(app: &mut App) -> Buffer {
    let backend = TestBackend::new(100, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

/// Header row text — row 0 holds the SessionHeader strip.
fn header_text(buf: &Buffer) -> String {
    row_text(buf, 0)
}

/// Scrape ALL rows below the header — used to assert the scrollback
/// contains (or does not contain) the `[compaction] …` notice line.
fn body_text(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 1..buf.area.height {
        s.push_str(&row_text(buf, y));
        s.push('\n');
    }
    s
}

// ── dual-transport harness (mirrors rpc037_cross_transport_parity.rs) ─────

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_service() -> (
    TempDir,
    Arc<SharedFspecService>,
    Arc<StubSessionManagerHandle>,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    (temp, service, stub)
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> (Arc<dyn FspecBackend>, Arc<dyn FspecBackend>) {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    (embedded, websocket)
}

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: CompactionComplete percent value renders directly in the badge and the notice
#[test]
fn compaction_complete_percent_value_renders_directly_in_badge_and_notice() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with fill_percentage 80) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(80),
        },
    ));

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12, turns_kept: 3 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 10000,
                compacted_tokens: 4000,
                compression_ratio: 60.0,
                turns_summarized: 12,
                turns_kept: 3,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);
    let body = body_text(&buf);

    // @step Then the SessionHeader text contains "[80%: COMPACTED 60%]"
    assert!(
        header.contains("[80%: COMPACTED 60%]"),
        "wire value 60.0 is already percent-removed and must render \
         '[80%: COMPACTED 60%]', got: {header:?}"
    );
    // @step And the scrollback contains a notice line containing "[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)"
    assert!(
        body.contains(
            "[compaction] 60.0% reduction (10000 \u{2192} 4000 tokens, 12 turns summarised)"
        ),
        "notice must render the wire percent directly, got body:\n{body}"
    );
}

/// Scenario: Regression — a 99.0 percent wire value renders COMPACTED 99% and never 9800%
#[test]
fn regression_99_percent_wire_value_renders_99_never_9800() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 100, compression_ratio: 99.0, turns_summarized: 20, turns_kept: 1 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 10000,
                compacted_tokens: 100,
                compression_ratio: 99.0,
                turns_summarized: 20,
                turns_kept: 1,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 99%"
    assert!(
        header.contains("COMPACTED 99%"),
        "a 99.0 percent wire value must render 'COMPACTED 99%', got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "9800"
    assert!(
        !header.contains("9800"),
        "the (1-99)*100 = -9800 inversion (sign-masked by .abs()) must \
         never appear, got: {header:?}"
    );
}

/// Scenario: The 0.0 sentinel renders as a zero-percent reduction
#[test]
fn zero_sentinel_renders_as_zero_percent_reduction() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 0, compacted_tokens: 0, compression_ratio: 0.0, turns_summarized: 0, turns_kept: 0 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 0,
                compacted_tokens: 0,
                compression_ratio: 0.0,
                turns_summarized: 0,
                turns_kept: 0,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);
    let body = body_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 0%"
    assert!(
        header.contains("COMPACTED 0%"),
        "the 0.0 sentinel must render 'COMPACTED 0%', got: {header:?}"
    );
    // @step And the scrollback contains a notice line containing "0.0% reduction"
    assert!(
        body.contains("0.0% reduction"),
        "the 0.0 sentinel notice must read '0.0% reduction', got body:\n{body}"
    );
    // @step And the scrollback does NOT contain a notice line containing "100.0% reduction"
    assert!(
        !body.contains("100.0% reduction"),
        "the pre-fix inversion turned 0.0 into '100.0% reduction'; that \
         must never happen, got body:\n{body}"
    );
}

/// Scenario: Stub parity — the canned CompactionResult round-trips as 50.0 percent on both transports
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stub_parity_canned_result_round_trips_as_50_percent_on_both_transports() {
    // @step Given a StubSessionManagerHandle serving both the embedded and WebSocket transports
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let session = embedded.create_session(None).await.expect("create_session");

    // @step When compact_session is called for the same session via each transport
    let em_res = embedded
        .compact_session(session.clone())
        .await
        .expect("em compact");
    let ws_res = websocket
        .compact_session(session.clone())
        .await
        .expect("ws compact");

    // @step Then both transports return CompactionResult with original_tokens 1000, compacted_tokens 500, compression_ratio 50.0, turns_summarized 4, turns_kept 2
    for (label, res) in [("embedded", &em_res), ("websocket", &ws_res)] {
        assert_eq!(res.original_tokens, 1000, "{label} original_tokens");
        assert_eq!(res.compacted_tokens, 500, "{label} compacted_tokens");
        assert!(
            (res.compression_ratio - 50.0).abs() < f64::EPSILON,
            "{label} compression_ratio must be 50.0 (percent removed), \
             got {}",
            res.compression_ratio
        );
        assert_eq!(res.turns_summarized, 4, "{label} turns_summarized");
        assert_eq!(res.turns_kept, 2, "{label} turns_kept");
    }

    // @step And formatting either result yields "[compaction] 50.0% reduction (1000 → 500 tokens, 4 turns summarised)"
    // Formatting is exercised through the real display pipeline: feed the
    // wire result into an App via CompactionComplete and read the notice
    // line out of the session's scrollback.
    let (mut app, _mock) = agent_app_with_single_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: em_res.clone(),
        },
    ));
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let body = body_text(&buf);
    assert!(
        body.contains(
            "[compaction] 50.0% reduction (1000 \u{2192} 500 tokens, 4 turns summarised)"
        ),
        "the canned 50.0 percent stub result must format as a 50.0% \
         reduction notice, got body:\n{body}"
    );
}

/// Scenario: Producer formula feeds the display pipeline end-to-end
#[test]
fn producer_formula_feeds_display_pipeline_end_to_end() {
    // @step Given the wire value is computed by the real producer formula compression_ratio(10000, 4000) * 100.0 equal to 60.0
    let wire_ratio = codelet_cli::interactive_helpers::compression_ratio(10000, 4000) * 100.0;
    assert!(
        (wire_ratio - 60.0).abs() < f64::EPSILON,
        "producer formula must yield 60.0 percent for 10000 → 4000, got {wire_ratio}"
    );

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete) carrying that wire value is dispatched for session "s-1"
    let (mut app, _mock) = agent_app_with_single_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 10000,
                compacted_tokens: 4000,
                compression_ratio: wire_ratio,
                turns_summarized: 12,
                turns_kept: 3,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);
    let body = body_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 60%"
    assert!(
        header.contains("COMPACTED 60%"),
        "the producer-derived 60.0 percent must render 'COMPACTED 60%', \
         got: {header:?}"
    );
    // @step And the scrollback contains a notice line containing "60.0% reduction"
    assert!(
        body.contains("60.0% reduction"),
        "the producer-derived 60.0 percent must format as '60.0% \
         reduction', got body:\n{body}"
    );
}
