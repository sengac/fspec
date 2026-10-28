use std::sync::Arc;

use codelet_fspec_tui::{Action, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitFocus};

mod common;

fn session(value: &str) -> SessionId {
    SessionId::new(value)
}

async fn settle(app: &mut codelet_fspec_tui::App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            assert!(task.await.is_ok());
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_an_older_update_cannot_resurrect_cleared_focus() {
    // @step Given a session whose card focus was explicitly cleared
    let (mut app, _) = common::test_app(Arc::new(common::MockBackend::new()));
    app.dispatch(Action::SessionCreated(session("first")));
    focus(&mut app, "first", 4, Some("AUTH-001"));
    focus(&mut app, "first", 5, None);

    // @step When an older card update or duplicate cleared update arrives
    focus(&mut app, "first", 4, Some("AUTH-001"));
    focus(&mut app, "first", 5, None);
    focus(&mut app, "first", 5, Some("AUTH-002"));
    app.dispatch(Action::WorkUnitAttached(session("first"), card("AUTH-001")));
    app.dispatch(Action::AttachSession("AUTH-001".into(), session("first")));

    // @step Then the session remains open without an active card
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    assert_eq!(chip_ids(&app), vec![None]);

    // @step And its board badge remains absent
    assert!(app.board_store().session_for("AUTH-001").is_none());
    assert!(app.board_store().session_for("AUTH-002").is_none());
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_a_reconnect_snapshot_restores_focus_without_local_attachment() {
    // @step Given a newly connected client observing two existing sessions
    let (mut app, _) = common::test_app(Arc::new(common::MockBackend::new()));
    app.dispatch(Action::SessionCreated(session("first")));
    app.dispatch(Action::SessionCreated(session("second")));

    // @step When it receives authoritative card focus for one session and cleared focus for the other
    focus(&mut app, "first", 3, Some("AUTH-001"));
    focus(&mut app, "second", 0, None);

    // @step Then only the first session has a card suffix and a board badge
    assert_eq!(chip_ids(&app), vec![Some("AUTH-001".to_string()), None]);
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&session("first"))
    );

    // @step And an older initial snapshot cannot replace a newer live focus update
    focus(&mut app, "first", 4, Some("AUTH-002"));
    focus(&mut app, "first", 3, Some("AUTH-001"));
    assert_eq!(chip_ids(&app), vec![Some("AUTH-002".to_string()), None]);
    assert!(app.board_store().session_for("AUTH-001").is_none());
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_several_sessions_can_share_a_card_without_losing_their_individual_focus() {
    // @step Given two open sessions focused on card "AUTH-001"
    let (mut app, _) = common::test_app(Arc::new(common::MockBackend::new()));
    for id in ["first", "second"] {
        app.dispatch(Action::SessionCreated(session(id)));
        focus(&mut app, id, 1, Some("AUTH-001"));
    }
    assert_eq!(
        chip_ids(&app),
        vec![Some("AUTH-001".into()), Some("AUTH-001".into())]
    );
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&session("first"))
    );

    // @step When the earliest session clears its focus
    focus(&mut app, "first", 2, None);

    // @step Then the other session remains focused on "AUTH-001"
    assert_eq!(chip_ids(&app), vec![None, Some("AUTH-001".into())]);

    // @step And the board badge still navigates to the remaining session
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&session("second"))
    );
    app.dispatch(Action::SessionRemoved(session("second")));
    assert!(app.board_store().session_for("AUTH-001").is_none());
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_explicit_assignment_into_an_existing_session_uses_the_authoritative_focus_path() {
    // @step Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    let mock = Arc::new(common::MockBackend::new());
    let (mut app, _) = common::test_app(mock.clone());
    app.dispatch(Action::SessionCreated(session("first")));
    focus(&mut app, "first", 0, Some("AUTH-001"));
    app.dispatch(Action::WorkUnitsLoaded(vec![
        codelet_rpc_types::WorkUnitInfo {
            id: "AUTH-002".into(),
            title: "Next".into(),
            status: "implementing".into(),
            work_type: "story".into(),
            description: None,
            estimate: None,
            epic: None,
            attachments: Vec::new(),
            last_state_change_at: None,
        },
    ]));

    // @step When the user explicitly assigns card "AUTH-002" to that session
    app.dispatch(Action::AttachWorkUnitToSession("AUTH-002".into()));
    settle(&mut app).await;

    // @step Then the backend and all card displays identify "AUTH-002"
    assert_eq!(
        mock.last_set_work_unit_context()
            .and_then(|(_, context)| context)
            .map(|context| context.id),
        Some("AUTH-002".into())
    );
    assert_eq!(chip_ids(&app), vec![Some("AUTH-002".into())]);
    assert!(app.board_store().session_for("AUTH-001").is_none());
    assert_eq!(
        app.board_store().session_for("AUTH-002"),
        Some(&session("first"))
    );

    // @step And a delayed acknowledgement for the previous card cannot replace the newer focus
    app.dispatch(Action::WorkUnitAttached(session("first"), card("AUTH-001")));
    focus(&mut app, "first", 0, Some("AUTH-001"));
    assert_eq!(chip_ids(&app), vec![Some("AUTH-002".into())]);
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_focused_card_changes_are_visible_on_board_agent_and_mux_surfaces() {
    // @step Given an open session with authoritative focus on card "AUTH-001"
    let (mut app, mut terminal) = common::test_app(Arc::new(common::MockBackend::new()));
    app.dispatch(Action::SessionCreated(session("first")));
    focus(&mut app, "first", 1, Some("AUTH-001"));

    // @step When its focus changes to card "AUTH-002" while each surface is rendered
    for (index, surface) in [
        codelet_fspec_tui::ViewMode::Board,
        codelet_fspec_tui::ViewMode::Agent,
        codelet_fspec_tui::ViewMode::Mux,
    ]
    .into_iter()
    .enumerate()
    {
        focus(&mut app, "first", index as u64 + 2, Some("AUTH-002"));
        if surface == codelet_fspec_tui::ViewMode::Mux {
            app.navigator_mut().mux.enable_with_config(
                codelet_fspec_tui::MuxConfig {
                    enabled: true,
                    ..Default::default()
                },
                codelet_fspec_tui::ViewMode::Board,
            );
        }
        app.navigator_mut().active_view = surface;
        let buffer = common::render_one_frame(&mut terminal, &mut app);
        let rendered: String = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();

        // @step Then the board agent and mux menu chips display "AUTH-002"
        assert!(
            rendered.contains("AUTH-002"),
            "missing chip on {surface:?}: {rendered}"
        );

        // @step And none of those surfaces displays "AUTH-001" as that session's focus
        assert!(
            !rendered.contains("AUTH-001"),
            "stale chip on {surface:?}: {rendered}"
        );
    }
}

fn focus(app: &mut codelet_fspec_tui::App, id: &str, revision: u64, value: Option<&str>) {
    app.dispatch(Action::WorkUnitFocusChanged(
        session(id),
        WorkUnitFocus {
            revision,
            context: value.map(card),
            source: "test".to_string(),
        },
    ));
}

fn chip_ids(app: &codelet_fspec_tui::App) -> Vec<Option<String>> {
    let store = app.agent_view_store();
    let active = codelet_fspec_tui::views::board::menu_snapshot::active_menu_session_ids(store);
    codelet_fspec_tui::views::board::menu_snapshot::chip_inputs(store, &active)
        .into_iter()
        .map(|chip| chip.wu_id)
        .collect()
}

fn card(value: &str) -> WorkUnitContext {
    WorkUnitContext {
        id: value.to_string(),
        title: value.to_string(),
        status: "implementing".to_string(),
    }
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_switching_focus_updates_the_menu_and_board_badge_together() {
    // @step Given an open session working on card "AUTH-001"
    let backend: Arc<dyn FspecBackend> = Arc::new(common::MockBackend::new());
    let (mut app, _terminal) = common::test_app(backend);
    app.dispatch(Action::SessionCreated(session("first")));
    focus(&mut app, "first", 1, Some("AUTH-001"));

    // @step When its authoritative focus changes to card "AUTH-002"
    focus(&mut app, "first", 2, Some("AUTH-002"));

    // @step Then its menu chip displays "AUTH-002" instead of "AUTH-001"
    let store = app.agent_view_store();
    let active = codelet_fspec_tui::views::board::menu_snapshot::active_menu_session_ids(store);
    let chips = codelet_fspec_tui::views::board::menu_snapshot::chip_inputs(store, &active);
    assert_eq!(chips[0].wu_id.as_deref(), Some("AUTH-002"));

    // @step And the board badge moves from "AUTH-001" to "AUTH-002"
    assert!(app.board_store().session_for("AUTH-001").is_none());
    assert_eq!(
        app.board_store().session_for("AUTH-002"),
        Some(&session("first"))
    );
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn scenario_a_cleared_session_does_not_inherit_another_sessions_card() {
    // @step Given a session working on "AUTH-001" and another session with no activity focus
    let backend: Arc<dyn FspecBackend> = Arc::new(common::MockBackend::new());
    let (mut app, mut terminal) = common::test_app(backend);
    app.dispatch(Action::SessionCreated(session("first")));
    app.dispatch(Action::WorkUnitAttached(session("first"), card("AUTH-001")));
    app.dispatch(Action::SessionCreated(session("second")));
    focus(&mut app, "second", 0, None);

    // @step When the cleared session is displayed with stale global card selection present
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.dispatch(Action::OpenAgentView(Some(session("second"))));
    let buffer = common::render_one_frame(&mut terminal, &mut app);
    let header: String = (0..buffer.area.width)
        .map(|column| buffer[(column, 0)].symbol())
        .collect();

    // @step Then its agent header and menu contain no work-unit suffix
    assert!(!header.contains("AUTH-001"), "unexpected header: {header}");
    let store = app.agent_view_store();
    let active = codelet_fspec_tui::views::board::menu_snapshot::active_menu_session_ids(store);
    let chips = codelet_fspec_tui::views::board::menu_snapshot::chip_inputs(store, &active);
    assert!(chips[1].wu_id.is_none());

    // @step And the other session remains focused on "AUTH-001"
    assert_eq!(
        store
            .work_unit_context_for(&session("first"))
            .map(|context| context.id.as_str()),
        Some("AUTH-001")
    );
}
