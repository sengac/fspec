//! Feature: spec/features/continue-cache-reconciliation.feature

use std::sync::atomic::Ordering;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{ContinueStateInfo, SessionId, StreamChunk};

#[path = "bug225/absolute_failure.rs"]
mod absolute_failure;
mod common;
#[path = "common/continue_backend.rs"]
mod continue_backend;
pub use common::MockBackend;
use continue_backend::{responses, settle, ContinueBackend};

fn session() -> SessionId {
    SessionId::new("continue-origin")
}

#[tokio::test]
async fn chrome_refresh_waits_for_an_already_submitted_continue_write() {
    // @step Given a hydrated enabled budget 300 with a budget 77 write still pending
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    settle(&mut app).await;
    backend.hold_write.store(true, Ordering::SeqCst);
    command(&mut app, "/continue 77");
    // @step When the session chrome refreshes before that write finishes
    app.dispatch(Action::ReattachSession(session()));
    for _attempt in 0..100 {
        tokio::task::yield_now().await;
    }
    let reads_while_pending = backend.reads.load(Ordering::SeqCst);
    backend.release_write.add_permits(1);
    settle(&mut app).await;
    // @step Then hydration waits for the write and retains enabled budget 77
    assert_eq!(reads_while_pending, 1, "refresh must wait for the setter");
    assert_eq!(
        app.agent_view_store().continue_state_for(&session()),
        (true, 77)
    );
}

fn command(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_owned()));
}

fn has_notice(app: &App, session: &SessionId, text: &str) -> bool {
    app.agent_view_store()
        .session_context_for(session)
        .is_some_and(|context| {
            context.scrollback.chunks().iter().any(|chunk| {
                chunk
                    .source
                    .as_ref()
                    .is_some_and(|source| source.text.contains(text))
            })
        })
}

fn live(app: &mut App, budget: u32) {
    app.dispatch(Action::ChunkReceived(
        session(),
        StreamChunk::ContinueStateUpdate {
            continue_state: ContinueStateInfo {
                enabled: true,
                budget,
                effective_budget: budget,
                nudges_used: 3,
                goal_active: false,
                done_rejections: 0,
                goal_cleared: false,
            },
        },
    ));
}

#[tokio::test]
async fn create_and_attach_hydrate_backend_continue_settings() {
    // @step Given backend sessions with enabled budget 300 and disabled budget 47
    for state in [(true, 300), (false, 47)] {
        for attach in [false, true] {
            let backend = Arc::new(ContinueBackend::new(state));
            let mut app = App::new(backend.clone());
            // @step When the TUI creates or attaches to those sessions without stream updates
            app.dispatch(if attach {
                Action::ReattachSession(session())
            } else {
                Action::SessionCreated(session())
            });
            settle(&mut app).await;
            // @step Then each session cache matches its backend settings without writing defaults
            assert_eq!(app.agent_view_store().continue_state_for(&session()), state);
            assert!(backend.writes().is_empty());
        }
    }
}

#[tokio::test]
async fn an_early_relative_command_waits_for_backend_state() {
    // @step Given an unknown continue cache and backend enabled budget 300
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    backend.hold_read.store(true, Ordering::SeqCst);
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    // @step When a bare toggle arrives before hydration completes
    command(&mut app, "/continue");
    backend.wait_for_read().await;
    assert!(backend.writes().is_empty());
    backend.release_read.add_permits(1);
    settle(&mut app).await;
    // @step Then the backend is disabled with budget 300 rather than enabled with budget 10
    assert_eq!(backend.state(), (false, 300));
    assert_eq!(
        app.agent_view_store().continue_state_for(&session()),
        (false, 300)
    );
}

#[tokio::test]
async fn a_failed_getter_refuses_guessed_relative_changes() {
    // @step Given an unknown continue cache whose backend getter fails
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    backend.fail_get.store(true, Ordering::SeqCst);
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    // @step When a bare toggle is submitted
    command(&mut app, "/continue");
    let actions = responses(&mut app).await;
    for action in actions {
        app.dispatch(action);
    }
    settle(&mut app).await;
    // @step Then no continue setter runs and the originating session receives an error
    assert!(has_notice(&app, &session(), "scripted getter failure"));
    assert!(backend.writes().is_empty());
    assert_eq!(backend.state(), (true, 300));
    assert_eq!(
        app.agent_view_store().continue_state_loaded_for(&session()),
        None
    );
    backend.fail_get.store(false, Ordering::SeqCst);
    command(&mut app, "/continue");
    settle(&mut app).await;
    assert_eq!(backend.state(), (false, 300));
}

#[tokio::test]
async fn a_failed_setter_reconciles_or_rolls_back_its_matching_cache() {
    // @step Given a hydrated enabled budget 300 and a failing continue setter
    for failed_read in [false, true] {
        let backend = Arc::new(ContinueBackend::new((true, 300)));
        let mut app = App::new(backend.clone());
        app.dispatch(Action::SessionCreated(session()));
        settle(&mut app).await;
        backend.fail_set.store(true, Ordering::SeqCst);
        backend.fail_get.store(failed_read, Ordering::SeqCst);
        // @step When a budget command fails with a successful or failed reconciliation getter
        command(&mut app, "/continue 77");
        let actions = responses(&mut app).await;
        assert!(actions
            .iter()
            .any(|action| format!("{action:?}").contains("scripted setter failure")));
        for action in actions {
            app.dispatch(action);
        }
        settle(&mut app).await;
        // @step Then the cache returns to enabled budget 300 and reports the failure
        assert!(has_notice(&app, &session(), "scripted setter failure"));
        assert_eq!(backend.reads.load(Ordering::SeqCst), 2);
        assert_eq!(
            app.agent_view_store().continue_state_for(&session()),
            (true, 300)
        );
    }
}

#[tokio::test]
async fn late_hydration_cannot_overwrite_commands_or_live_updates() {
    // @step Given a pending hydration response with enabled budget 300
    for streamed in [false, true] {
        let backend = Arc::new(ContinueBackend::new((true, 300)));
        let mut app = App::new(backend);
        app.dispatch(Action::SessionCreated(session()));
        let delayed = responses(&mut app).await;
        // @step When a newer budget command or live continue update arrives
        if streamed {
            live(&mut app, 77);
        } else {
            command(&mut app, "/continue 77");
        }
        for action in delayed {
            app.dispatch(action);
        }
        settle(&mut app).await;
        // @step Then the late hydration leaves the newer state intact
        assert_eq!(
            app.agent_view_store().continue_state_for(&session()),
            (true, 77)
        );
    }
}

#[tokio::test]
async fn responses_remain_bound_to_their_originating_session() {
    // @step Given hydration and a relative command pending for one session
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    command(&mut app, "/continue");
    let delayed = responses(&mut app).await;
    // @step When focus switches to a second session before the response arrives
    let other = SessionId::new("other-session");
    app.dispatch(Action::SessionCreated(other.clone()));
    for action in delayed {
        app.dispatch(action);
    }
    settle(&mut app).await;
    // @step Then only the originating session is hydrated and toggled
    assert_eq!(app.agent_view_store().current_session(), Some(&other));
    assert_eq!(
        app.agent_view_store().continue_state_for(&session()),
        (false, 300)
    );
    assert!(backend.writes().iter().all(|(id, _, _)| id == &session()));
}

#[tokio::test]
async fn late_setter_failure_cannot_roll_back_newer_state() {
    // @step Given a pending failing budget command
    for streamed in [false, true] {
        let backend = Arc::new(ContinueBackend::new((true, 300)));
        let mut app = App::new(backend.clone());
        app.dispatch(Action::SessionCreated(session()));
        settle(&mut app).await;
        backend.fail_set.store(true, Ordering::SeqCst);
        command(&mut app, "/continue 50");
        let delayed = responses(&mut app).await;
        backend.fail_set.store(false, Ordering::SeqCst);
        // @step When a newer command or live continue update changes the cache
        if streamed {
            live(&mut app, 77);
        } else {
            command(&mut app, "/continue 77");
        }
        for action in delayed {
            app.dispatch(action);
        }
        settle(&mut app).await;
        // @step Then the older failure does not roll back the newer state
        assert_eq!(
            app.agent_view_store().continue_state_for(&session()),
            (true, 77)
        );
    }
}

#[tokio::test]
async fn rapid_continue_commands_preserve_submission_order() {
    // @step Given an unknown continue cache and backend enabled budget 300
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    // @step When two bare toggles and a budget command arrive before hydration
    command(&mut app, "/continue");
    command(&mut app, "/continue");
    command(&mut app, "/continue 77");
    settle(&mut app).await;
    // @step Then the writes run in command order and finish enabled with budget 77
    let writes = backend.writes();
    let pairs: Vec<_> = writes
        .iter()
        .map(|(_, enabled, budget)| (*enabled, *budget))
        .collect();
    assert_eq!(pairs, vec![(false, 300), (true, 10), (true, 77)]);
    assert_eq!(backend.state(), (true, 77));
    assert_eq!(
        app.agent_view_store().continue_state_for(&session()),
        (true, 77)
    );
}
