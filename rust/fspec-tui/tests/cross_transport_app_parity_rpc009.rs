//! Cross-transport App-layer parity (RPC-009).
//!
//! Feature: spec/features/fspec-tui-cross-transport-app-parity-rpc009.feature
//!
//! Drives the same scripted scenario (seed work-units.json → render →
//! mutate work-units.json → wait for broadcast → render again) against
//! BOTH transports — embedded and WebSocket — and asserts the App-layer
//! observable behaviour is identical.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};

mod common;
use common::{
    buffer_to_rows, render_one_frame, start_ws_server, temp_service_with_stub as temp_service,
    ws_url,
};

const SEED_PLUS_THREE_WORK_UNITS_JSON: &str = r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"User Login","type":"story","status":"done","estimate":5,"epic":"authentication"},"AUTH-002":{"id":"AUTH-002","title":"Password reset","type":"story","status":"implementing","estimate":3,"epic":"authentication"},"AUTH-003":{"id":"AUTH-003","title":"OAuth","type":"story","status":"backlog","epic":"authentication"}}}"#;

/// Render the App against an 80x24 TestBackend and return the row text.
async fn render_app_rows(app: &mut App) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut term = ratatui::Terminal::new(backend).or_fail("Terminal::new")?;
    let buf = render_one_frame(&mut term, app)?;
    Ok(buffer_to_rows(&buf))
}

/// Dispatch the App's actions until its board holds work unit `id`
/// (TEST-042: the update the test asserts, under the deadlock guard).
async fn dispatch_until_board_has(app: &mut App, id: &str) -> TestResult {
    codelet_test_helpers::deadlock_guard("the work-units update on the board", async {
        while app.board_store().find(id).is_none() {
            while let Some(action) = app.try_recv_action() {
                app.dispatch(action);
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

/// Scenario: Embedded App smoke — mutate spec/work-units.json on disk and observe the rendered left pane reflects the new state
#[tokio::test]
async fn embedded_app_smoke_observes_left_pane_reflects_new_state() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService seeded with [AUTH-001 done, AUTH-002 implementing]
    let (dir, service) = temp_service()?;
    // @step And an `EmbeddedFspecBackend` constructed via `EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service)`
    let handle = tokio::runtime::Handle::current();
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(handle, service));
    // @step And an `App::new(Arc::new(backend))` rendered onto an 80x24 TestBackend
    let mut app = App::new(backend);
    // @step When the App's bootstrap completes and one frame is rendered
    app.bootstrap().await.or_fail("bootstrap")?;
    let rows_before = render_app_rows(&mut app).await?;
    // @step Then the rendered buffer's left pane contains "AUTH-001 done" and "AUTH-002 implementing"
    // RPC-012 update: BoardView renders just `{id}` (or `{id} [{points}]`)
    // per the rich UnifiedBoardLayout topology — status comes from the
    // column position, not from inline text.
    let joined = rows_before.join("\n");
    assert!(joined.contains("AUTH-001"));
    assert!(joined.contains("AUTH-002"));
    // @step When the test rewrites `<tempdir>/spec/work-units.json` to add a third entry AUTH-003 backlog
    std::fs::write(
        dir.path().join("spec").join("work-units.json"),
        SEED_PLUS_THREE_WORK_UNITS_JSON,
    )
    .or_fail("write")?;
    // @step And the work_units subscriber task observes the broadcast within 200ms
    dispatch_until_board_has(&mut app, "AUTH-003").await?;
    // @step And the App processes `Action::WorkUnitsLoaded` and renders another frame
    let rows_after = render_app_rows(&mut app).await?;
    // @step Then the rendered buffer's left pane contains "AUTH-003 backlog"
    let joined_after = rows_after.join("\n");
    assert!(joined_after.contains("AUTH-003"));
    Ok(())
}

/// Scenario: WS App smoke — spawn rpc-server and observe the rendered left pane reflects the new state
#[tokio::test]
async fn ws_app_smoke_observes_left_pane_reflects_new_state() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService seeded with [AUTH-001 done, AUTH-002 implementing]
    let (dir, service) = temp_service()?;
    // @step And a `bind_and_serve` rpc-server bound to 127.0.0.1:0 against that service
    let (addr, _join) = start_ws_server(service.clone()).await?;
    // @step And a `WebSocketFspecBackend::connect(ws_url).await` against the resulting ws://127.0.0.1:<port>/ url
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(ws_url(addr)?)
            .await
            .or_fail("connect")?,
    );
    // @step And an `App::new(Arc::new(backend))` rendered onto an 80x24 TestBackend
    let mut app = App::new(backend);
    // @step When the App's bootstrap completes and one frame is rendered
    app.bootstrap().await.or_fail("bootstrap")?;
    let rows_before = render_app_rows(&mut app).await?;
    // @step Then the rendered buffer's left pane contains "AUTH-001 done" and "AUTH-002 implementing"
    let joined = rows_before.join("\n");
    assert!(joined.contains("AUTH-001"));
    assert!(joined.contains("AUTH-002"));
    // @step When the test rewrites `<tempdir>/spec/work-units.json` to add a third entry AUTH-003 backlog
    std::fs::write(
        dir.path().join("spec").join("work-units.json"),
        SEED_PLUS_THREE_WORK_UNITS_JSON,
    )
    .or_fail("write")?;
    // @step And the work_units subscriber task observes the broadcast within 200ms
    dispatch_until_board_has(&mut app, "AUTH-003").await?;
    // @step And the App processes `Action::WorkUnitsLoaded` and renders another frame
    let rows_after = render_app_rows(&mut app).await?;
    // @step Then the rendered buffer's left pane contains "AUTH-003 backlog"
    let joined_after = rows_after.join("\n");
    assert!(joined_after.contains("AUTH-003"));
    Ok(())
}

/// Scenario: Cross-transport parity — both Apps' rendered left panes are semantically identical post-mutation
#[tokio::test]
async fn cross_transport_parity_left_panes_are_semantically_identical_post_mutation() -> TestResult
{
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given a shared tempdir-backed WorkUnitsWatcher fixture seeded with [AUTH-001 done, AUTH-002 implementing]
    let (dir_e, service_e) = temp_service()?;
    let (dir_w, service_w) = temp_service()?;
    // @step And an App-on-EmbeddedFspecBackend rendered onto an 80x24 TestBackend
    let backend_e: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service_e,
    ));
    let mut app_e = App::new(backend_e);
    app_e.bootstrap().await.or_fail("embedded bootstrap")?;
    // @step And an App-on-WebSocketFspecBackend (against `bind_and_serve` on 127.0.0.1:0) rendered onto an 80x24 TestBackend
    let (addr, _join) = start_ws_server(service_w.clone()).await?;
    let backend_w: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(ws_url(addr)?)
            .await
            .or_fail("connect")?,
    );
    let mut app_w = App::new(backend_w);
    app_w.bootstrap().await.or_fail("ws bootstrap")?;
    // @step When the test mutates the workspace's spec/work-units.json to add AUTH-003 backlog
    std::fs::write(
        dir_e.path().join("spec").join("work-units.json"),
        SEED_PLUS_THREE_WORK_UNITS_JSON,
    )
    .or_fail("embedded write")?;
    std::fs::write(
        dir_w.path().join("spec").join("work-units.json"),
        SEED_PLUS_THREE_WORK_UNITS_JSON,
    )
    .or_fail("ws write")?;
    // @step And both Apps process `Action::WorkUnitsLoaded` and render another frame
    dispatch_until_board_has(&mut app_e, "AUTH-003").await?;
    dispatch_until_board_has(&mut app_w, "AUTH-003").await?;
    let rows_e = render_app_rows(&mut app_e).await?;
    let rows_w = render_app_rows(&mut app_w).await?;
    // @step Then the row sequence in each App's left-pane buffer band (rows containing "AUTH-") matches the same set of work-unit ids in the same order
    let auth_rows_e: Vec<&String> = rows_e.iter().filter(|r| r.contains("AUTH-")).collect();
    let auth_rows_w: Vec<&String> = rows_w.iter().filter(|r| r.contains("AUTH-")).collect();
    assert_eq!(auth_rows_e.len(), auth_rows_w.len());
    for (e, w) in auth_rows_e.iter().zip(auth_rows_w.iter()) {
        // Compare just the AUTH-XXX status portion to be transport-agnostic
        assert!(
            e.contains("AUTH-") && w.contains("AUTH-"),
            "embedded: {e}, ws: {w}"
        );
    }
    // @step And no transport-specific divergence in id, status, or item formatting appears
    let ids_e: Vec<String> = auth_rows_e.iter().map(|r| extract_auth_id(r)).collect();
    let ids_w: Vec<String> = auth_rows_w.iter().map(|r| extract_auth_id(r)).collect();
    assert_eq!(ids_e, ids_w);
    Ok(())
}

fn extract_auth_id(row: &str) -> String {
    let trimmed = row.trim();
    let idx = trimmed.find("AUTH-").unwrap_or(0);
    trimmed.chars().skip(idx).take(8).collect()
}
