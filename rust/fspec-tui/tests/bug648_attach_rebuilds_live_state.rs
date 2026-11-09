//! BUG-648: every attach to a running session rebuilds its exact live state.
//!
//! Feature: spec/features/attaching-to-a-running-session-rebuilds-its-live-state.feature
//!
//! A real daemon (SessionManager + SharedFspecService) is driven directly on
//! its BackgroundSession. One TUI is attached from before the state was
//! reached; a second attaches afterwards by the path under test. The second
//! must show exactly what the first shows (the LiveView digest), on the
//! embedded and the WebSocket transport. Waits are on state (a sentinel
//! notice after the scripted events), under the deadlock guard (TEST-042).

#[path = "bug648/support.rs"]
mod support;

mod common;

use std::sync::Arc;

use codelet_fspec_tui::daemons::SwitchedBackend;
use codelet_fspec_tui::{Action, App, WebSocketFspecBackend};
use codelet_rpc_types::{
    FspecRequest, NotificationSeverity, SessionId, SessionStatus, StreamChunk,
};
use codelet_test_helpers::{OrFail, TestResult};
use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn opening_a_running_session_with_resume_shows_its_spinner_its_running_tool_s_output_and_its_token_counts(
) -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    for transport in TRANSPORTS {
        let id = daemon.create_session("resume").await?;
        let session = daemon.session(&id)?;
        // @step And a TUI has been attached to S since before it started
        let mut first = App::new(daemon.backend(transport).await?);
        first.bootstrap().await?;
        // The startup restore settles first, so the attach is the last focus change.
        settle(&mut first).await?;
        first.dispatch(Action::AttachToSession(id.clone()));
        settle(&mut first).await?;
        // @step Given session S is Running with a Bash tool call that has streamed 'line 1'
        // @step And S has reported 1200 input and 300 output tokens and a context fill of 40%
        running_tool(&session);
        sentinel(&session, "bug648 resume ready");
        wait_for_marker(&mut first, &id, "bug648 resume ready").await?;

        // @step When a second TUI opens S with /resume over either transport
        let mut second = App::new(daemon.backend(transport).await?);
        second.bootstrap().await?;
        // The startup restore settles first, so the attach is the last focus change.
        settle(&mut second).await?;
        second.dispatch(Action::AttachToSession(id.clone()));
        wait_for_marker(&mut second, &id, "bug648 resume ready").await?;

        // @step Then the second TUI shows the Thinking spinner and the Bash tool card with 'line 1'
        let rows = render_rows(&mut second, 60);
        assert!(
            rows.iter().any(|row| row.contains("Thinking")),
            "{transport:?}: the Thinking spinner is shown: {rows:#?}"
        );
        let view = live_view(&second, &id);
        assert_eq!(view.status, Some(SessionStatus::Running), "{transport:?}");
        assert!(
            view.transcript.iter().any(|text| text.contains("line 1")),
            "{transport:?}: the running tool's output is shown: {:#?}",
            view.transcript
        );
        // @step And its view of S equals the first TUI's, token and context-fill counters included
        assert!(
            view.tokens.is_some(),
            "{transport:?}: token counters are shown"
        );
        assert_eq!(view, live_view(&first, &id), "{transport:?}");
    }
    daemon.stop();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn restoring_the_active_session_at_startup_shows_its_pending_question() -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    for transport in TRANSPORTS {
        let id = daemon.create_session("restore").await?;
        let session = daemon.session(&id)?;
        let mut first = App::new(daemon.backend(transport).await?);
        first.bootstrap().await?;
        settle(&mut first).await?;
        // @step Given session S asks 'Which auth library?' and is Paused
        ask(&session, "Which auth library?");
        sentinel(&session, "bug648 restore ready");
        wait_for_marker(&mut first, &id, "bug648 restore ready").await?;
        // @step And S is the daemon's active session
        let backend = daemon.backend(transport).await?;
        backend.set_active_session(id.clone()).await?;

        // @step When a TUI starts and restores S over either transport
        let mut second = App::new(backend);
        second.bootstrap().await?;
        wait_for_marker(&mut second, &id, "bug648 restore ready").await?;
        assert_eq!(second.agent_view_store().current_session(), Some(&id));

        // @step Then it shows the inline question 'Which auth library?' with S's request id
        let route = session
            .pending_question_route()
            .or_fail("S has a pending question")?;
        let view = live_view(&second, &id);
        assert_eq!(
            view.question,
            Some((route.request_id.clone(), "Which auth library?".to_string())),
            "{transport:?}"
        );
        assert_eq!(view.status, Some(SessionStatus::Paused), "{transport:?}");
        // @step And its view of S equals that of a TUI attached to S since before it asked
        assert_eq!(view, live_view(&first, &id), "{transport:?}");
    }
    daemon.stop();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn switching_to_the_daemon_that_hosts_a_compacting_session_shows_its_compaction_progress(
) -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon_a = Daemon::start().await?;
    let daemon_b = Daemon::start().await?;
    for transport in TRANSPORTS {
        let id = daemon_b.create_session("compacting").await?;
        let session = daemon_b.session(&id)?;
        let mut first = App::new(daemon_b.backend(transport).await?);
        first.bootstrap().await?;
        settle(&mut first).await?;
        // @step Given session S on daemon B is Compacting with progress 'summarizing 2 of 5'
        session.set_status(SessionStatus::Compacting);
        session.update_compaction_progress("summarizing".into(), 2, 5);
        sentinel(&session, "bug648 compaction ready");
        wait_for_marker(&mut first, &id, "bug648 compaction ready").await?;

        // @step And a TUI is connected to daemon A
        let mut second = App::new(daemon_a.backend(transport).await?);
        second.bootstrap().await?;
        settle(&mut second).await?;

        // @step When the TUI switches to daemon B over either transport
        second.dispatch(Action::DaemonSwitched(SwitchedBackend {
            name: "daemon-b".into(),
            backend: daemon_b.backend(transport).await?,
        }));
        wait_for_marker(&mut second, &id, "bug648 compaction ready").await?;

        // @step Then S's tab shows the Compacting spinner and the progress 'summarizing 2 of 5'
        let view = live_view(&second, &id);
        assert_eq!(
            view.status,
            Some(SessionStatus::Compacting),
            "{transport:?}"
        );
        let progress = view
            .compaction
            .clone()
            .or_fail("compaction progress is shown")?;
        assert!(
            progress.contains("summarizing") && progress.contains('2') && progress.contains('5'),
            "{transport:?}: {progress}"
        );
        // @step And its view of S equals that of a TUI connected to daemon B since before S started compacting
        assert_eq!(view, live_view(&first, &id), "{transport:?}");
    }
    daemon_a.stop();
    daemon_b.stop();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn switching_to_the_workspace_of_a_session_waiting_on_an_approval_shows_the_approval(
) -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    let second_workspace = tempfile::tempdir()?;
    std::fs::create_dir_all(second_workspace.path().join("spec"))?;
    std::fs::write(
        second_workspace.path().join("spec/work-units.json"),
        "{\"workUnits\":{}}",
    )?;
    let w2 = daemon.service.add_workspace(second_workspace.path())?.path;
    daemon.manager.set_default_model(MODEL)?;
    for transport in TRANSPORTS {
        let backend = daemon.backend(transport).await?;
        // @step Given session S in workspace W2 waits on an approval
        let id = backend
            .create_session_in_workspace(w2.clone(), None)
            .await?;
        let session = daemon.session(&id)?;
        let mut first = App::new(daemon.backend(transport).await?);
        first.bootstrap().await?;
        settle(&mut first).await?;
        choose_workspace(&mut first, &w2).await?;
        ask_approval(&session, "Run ./deploy.sh?");
        sentinel(&session, "bug648 workspace ready");
        wait_for_marker(&mut first, &id, "bug648 workspace ready").await?;

        // @step And a TUI that connected after S asked shows workspace W1
        let mut second = App::new(daemon.backend(transport).await?);
        second.bootstrap().await?;
        settle(&mut second).await?;
        assert_ne!(
            second
                .board_store()
                .selected_workspace()
                .map(|entry| entry.path.clone()),
            Some(w2.clone())
        );

        // @step When the TUI switches to workspace W2
        choose_workspace(&mut second, &w2).await?;
        wait_for_marker(&mut second, &id, "bug648 workspace ready").await?;

        // @step Then S's tab shows the approval prompt with its request id
        let route = session
            .pending_pause_route()
            .or_fail("S has a pending approval")?;
        let view = live_view(&second, &id);
        assert_eq!(
            view.approval.as_ref().map(|(request, _)| request.clone()),
            Some(route.request_id.clone()),
            "{transport:?}"
        );
        // @step And its view of S equals that of a TUI attached to S since before it asked
        assert_eq!(view, live_view(&first, &id), "{transport:?}");
    }
    daemon.stop();
    Ok(())
}

/// Choose `path` in the board's workspace dropdown.
async fn choose_workspace(app: &mut App, path: &str) -> TestResult {
    wait_until(app, "the workspace list", |app| {
        (0..16).any(|row| {
            app.board_store()
                .workspace_dropdown_entry(Some(row))
                .is_some_and(|entry| entry.path == path)
        })
    })
    .await?;
    let row = (0..16)
        .find(|row| {
            app.board_store()
                .workspace_dropdown_entry(Some(*row))
                .is_some_and(|entry| entry.path == path)
        })
        .or_fail("the workspace is listed")?;
    app.dispatch(Action::ChooseWorkspace(Some(row)));
    settle(app).await
}

// The exec-stdin slot holds a request only while its exec session is live,
// so the scenario runs a real long-running exec command (under another agent
// id, so no quiet detector of S's replaces the scripted request).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_mux_pane_showing_a_session_with_a_pending_stdin_prompt_shows_the_prompt() -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    for transport in TRANSPORTS {
        let id = daemon.create_session("stdin").await?;
        let session = daemon.session(&id)?;
        let mut first = App::new(daemon.backend(transport).await?);
        first.bootstrap().await?;
        // The startup restore settles first, so the attach is the last focus change.
        settle(&mut first).await?;
        first.dispatch(Action::AttachToSession(id.clone()));
        settle(&mut first).await?;
        // @step Given session S has a pending exec-stdin prompt for 'npm login'
        session.set_status(SessionStatus::Running);
        let (exec, exec_id) = live_exec().await?;
        session.set_exec_stdin_request(Some(codelet_tools::unified_exec::ExecStdinRequest {
            exec_session_id: exec_id.clone(),
            command: "npm login".into(),
            quiet_seconds: 3,
            ts_ms: 1,
        }))?;
        sentinel(&session, "bug648 stdin ready");
        wait_for_marker(&mut first, &id, "bug648 stdin ready").await?;

        // @step When mux mode shows S in a pane of a TUI that connected after the prompt, over either transport
        let mut second = App::new(daemon.backend(transport).await?);
        second.bootstrap().await?;
        // The startup restore settles first, so the attach is the last focus change.
        settle(&mut second).await?;
        second.dispatch(Action::AttachToSession(id.clone()));
        wait_for_marker(&mut second, &id, "bug648 stdin ready").await?;
        wait_until(&mut second, "S is the current session", |app| {
            app.agent_view_store().current_session() == Some(&id)
        })
        .await?;
        render_rows(&mut second, 1);
        second.dispatch(Action::InputSubmitted(id.clone(), "/mux on".into()));
        settle(&mut second).await?;
        assert_eq!(
            second.active_view(),
            codelet_fspec_tui::views::ViewMode::Mux,
            "{transport:?}: {:#?}",
            transcript(&second, &id)
        );

        // @step Then the pane shows the stdin prompt for 'npm login' and S's status
        render_rows(&mut second, 1);
        second
            .navigator_mut()
            .mux
            .focus_agent_pane_for_session(&id)
            .or_fail("S is shown in an agent pane")?;
        let rows = render_rows(&mut second, 60);
        assert!(
            rows.iter().any(|row| row.contains("npm login")),
            "{transport:?}: the stdin prompt is shown: {rows:#?}"
        );
        let view = live_view(&second, &id);
        assert_eq!(view.status, Some(SessionStatus::Running), "{transport:?}");
        assert_eq!(view.exec_stdin, Some(format!("{exec_id} npm login")));
        // @step And its view of S equals that of a TUI attached to S since before the prompt
        assert_eq!(view, live_view(&first, &id), "{transport:?}");
        close_exec(&exec, &exec_id).await?;
    }
    daemon.stop();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn events_missed_while_the_websocket_was_down_come_back_once_after_the_reconnect(
) -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    let id = daemon.create_session("reconnect").await?;
    let session = daemon.session(&id)?;
    // @step Given a TUI shows session S over the WebSocket
    let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();
    let url = common::ws_url(daemon.address)?;
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone()).await?;
    let mut tui = App::with_action_bus(Arc::new(backend), action_tx, action_rx);
    tui.bootstrap().await?;
    // The startup restore settles first, so the attach is the last focus change.
    settle(&mut tui).await?;
    tui.dispatch(Action::AttachToSession(id.clone()));
    let mut stayed = App::new(daemon.backend(Transport::Embedded).await?);
    stayed.bootstrap().await?;
    sentinel(&session, "bug648 connected");
    wait_for_marker(&mut tui, &id, "bug648 connected").await?;
    wait_for_marker(&mut stayed, &id, "bug648 connected").await?;

    // @step And the WebSocket is down while S goes Running, runs a tool, goes Idle and reports new token counts
    daemon.stop();
    wait_for_marker(&mut tui, &id, "Reconnecting").await?;
    session.set_status(SessionStatus::Running);
    session.handle_output(tool_call("tool-9"));
    session.handle_output(tool_output("tool-9", "compiled 12 crates\n"));
    session.handle_output(tool_result("tool-9", "build finished"));
    session.handle_output(tokens(5000, 800));
    session.set_status(SessionStatus::Idle);
    sentinel(&session, "bug648 after the drop");

    // @step When the WebSocket reconnects
    let address = format!("127.0.0.1:{}", daemon.address.port());
    let (_, _stats, server) =
        codelet_rpc_server::bind_and_serve(&address, daemon.service.clone()).await?;
    wait_for_marker(&mut tui, &id, "bug648 after the drop").await?;
    wait_for_marker(&mut stayed, &id, "bug648 after the drop").await?;

    // @step Then the TUI shows S Idle with the finished tool card once and the new token counts
    let view = live_view(&tui, &id);
    assert_eq!(view.status, Some(SessionStatus::Idle));
    assert_eq!(
        view.transcript
            .iter()
            .filter(|text| text.contains("compiled 12 crates"))
            .count(),
        1,
        "{:#?}",
        view.transcript
    );
    assert!(view
        .tokens
        .as_deref()
        .is_some_and(|tokens| tokens.contains("5000")));
    // @step And its view of S equals that of a TUI that stayed connected
    assert_eq!(view, live_view(&stayed, &id));
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_replayed_approval_that_was_answered_leaves_no_prompt() -> TestResult {
    let root = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment::isolate(root.path())?;
    let daemon = Daemon::start().await?;
    for transport in TRANSPORTS {
        let id = daemon.create_session("answered").await?;
        let session = daemon.session(&id)?;
        let backend = daemon.backend(transport).await?;
        // @step Given session S's journal holds an approval that was answered, then Running, then Idle
        ask_approval(&session, "Delete build/?");
        let route = session
            .pending_pause_route()
            .or_fail("S has a pending approval")?;
        backend
            .pause_confirm(id.clone(), route.request_id, true)
            .await?;
        session.set_status(SessionStatus::Idle);
        sentinel(&session, "bug648 answered");

        // @step When a TUI attaches to S
        let mut tui = App::new(backend);
        tui.bootstrap().await?;
        // The startup restore settles first, so the attach is the last focus change.
        settle(&mut tui).await?;
        tui.dispatch(Action::AttachToSession(id.clone()));
        wait_for_marker(&mut tui, &id, "bug648 answered").await?;

        // @step Then it shows S Idle with no approval prompt
        let view = live_view(&tui, &id);
        assert_eq!(view.status, Some(SessionStatus::Idle), "{transport:?}");
        assert_eq!(view.approval, None, "{transport:?}");
        assert_eq!(view.question, None, "{transport:?}");
    }
    daemon.stop();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_replayed_fspec_command_request_is_not_run_again() -> TestResult {
    let mock = Arc::new(common::MockBackend::default());
    let id = SessionId::new("s-fspec");
    let request = |tool: &str| {
        StreamChunk::fspec_command_request(FspecRequest {
            command: "list-work-units".into(),
            args_json: "{}".into(),
            project_root: String::new(),
            tool_call_id: tool.into(),
        })
    };
    let notice =
        |text: &str| StreamChunk::user_notification(text.into(), NotificationSeverity::Info);
    // @step Given session S's journal holds an fspec command request that was already answered
    mock.set_buffered_output(vec![request("tool-answered"), notice("bug648 replayed")]);
    let mut tui = App::new(mock.clone());
    tui.bootstrap().await?;

    // @step When a TUI attaches to S
    tui.dispatch(Action::AttachToSession(id.clone()));
    wait_for_marker(&mut tui, &id, "bug648 replayed").await?;

    // @step Then it sends no fspec result for that request
    assert_eq!(mock.send_fspec_result_calls(), 0);
    // A request that arrives live still runs, once.
    for chunk in [request("tool-live"), notice("bug648 live")] {
        mock.push_buffered_output(chunk.clone());
        mock.push_chunk(id.clone(), chunk);
    }
    wait_for_marker(&mut tui, &id, "bug648 live").await?;
    wait_until(&mut tui, "the live request's result", |_| {
        mock.send_fspec_result_calls() > 0
    })
    .await?;
    settle(&mut tui).await?;
    assert_eq!(mock.send_fspec_result_calls(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_answered_in_one_chain_member_s_transcript_stays_closed_when_another_member_s_copy_arrives_later(
) -> TestResult {
    use codelet_rpc_types::{RequestHolder, RequestKind, RequestRoute, RequestStatus};
    let mock = Arc::new(common::MockBackend::default());
    let (bot, sub) = (SessionId::new("s-bot"), SessionId::new("s-sub"));
    let mut tui = App::new(mock.clone());
    for session in [&bot, &sub] {
        mock.script_pending_pause(session.clone(), None);
        mock.script_pending_hitl(session.clone(), None);
        tui.dispatch(Action::SessionCreated(session.clone()));
        settle(&mut tui).await?;
    }
    // @step Given sub-agent #2 of bot #1 asks a question held by the user
    let route = RequestRoute {
        request_id: "request-7".into(),
        origin: sub.clone(),
        kind: RequestKind::Question,
        chain: vec![sub.clone(), bot.clone()],
        holder: RequestHolder::User,
    };
    let event = |status: RequestStatus| StreamChunk::InteractionRequestChanged {
        route: route.clone(),
        status,
    };
    // @step And bot #1's transcript delivers the request's Pending event twice and then its Answered event
    for status in [
        RequestStatus::Pending,
        RequestStatus::Pending,
        RequestStatus::Answered {
            by: RequestHolder::User,
        },
    ] {
        tui.dispatch(Action::ChunkReceived(bot.clone(), event(status)));
        settle(&mut tui).await?;
    }
    // @step When sub-agent #2's transcript delivers its Pending copy of the request afterwards
    tui.dispatch(Action::ChunkReceived(
        sub.clone(),
        event(RequestStatus::Pending),
    ));
    settle(&mut tui).await?;
    // @step Then bot #1's view shows no relayed question and no waiting marker for it
    let store = tui.agent_view_store();
    assert!(store.relayed_request_for(&bot).is_none());
    assert!(store.waiting_requests_for(&bot).is_empty());
    assert!(!store.waits_on_request(&bot));
    Ok(())
}
