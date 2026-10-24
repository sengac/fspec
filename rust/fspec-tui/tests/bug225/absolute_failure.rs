//! Feature: spec/features/continue-cache-reconciliation.feature

use super::{command, session, settle, Action, App, Arc, ContinueBackend, Ordering};

#[tokio::test]
async fn an_unconfirmed_absolute_command_remains_unknown_after_failed_reconciliation() {
    // @step Given an unknown continue cache whose getter and setter both fail
    let backend = Arc::new(ContinueBackend::new((true, 300)));
    backend.fail_get.store(true, Ordering::SeqCst);
    backend.fail_set.store(true, Ordering::SeqCst);
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session()));
    // @step When a numeric budget command fails and a bare toggle follows
    command(&mut app, "/continue 77");
    settle(&mut app).await;
    assert_eq!(
        app.agent_view_store().continue_state_loaded_for(&session()),
        None
    );
    command(&mut app, "/continue");
    settle(&mut app).await;
    // @step Then the cache remains unknown and the toggle never submits a guessed state
    assert_eq!(
        app.agent_view_store().continue_state_loaded_for(&session()),
        None
    );
    assert_eq!(backend.writes(), vec![(session(), true, 77)]);
    assert_eq!(backend.state(), (true, 300));
}
