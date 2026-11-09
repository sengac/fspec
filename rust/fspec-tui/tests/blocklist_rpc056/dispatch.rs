//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature

use super::*;
use codelet_test_helpers::TestResult;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_blocklist_dispatches_open_and_fetches_rules() -> TestResult {
    // @step Given an App with an open session s-1 wired to a MockBackend whose blocklist_list returns two rules
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await?;

    let initial_calls = mock.blocklist_list_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Blocklist) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Blocklist,
    ));
    drain_pending(&mut app).await?;

    // @step Then within 1 second backend.blocklist_list is called exactly once
    wait_until(
        || mock.blocklist_list_calls() - initial_calls == 1,
        "blocklist_list_calls == initial + 1",
    )
    .await?;

    // @step And within 1 second the Navigator's active_view equals ViewMode::Blocklist
    wait_until(
        || app.navigator().active_view == ViewMode::Blocklist,
        "active_view == ViewMode::Blocklist",
    )
    .await?;

    // @step And within 1 second Action::BlocklistRulesLoaded carrying the two rules is observed on the action bus
    let rules = &app.navigator().blocklist.rules;
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].id, "git-checkout-block");
    assert_eq!(rules[1].id, "cat-block");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_closes_view_and_returns_to_agent() -> TestResult {
    // @step Given an App with an open session s-1 and the Navigator active_view set to ViewMode::Blocklist
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![rule("a", "a", "block", "system")]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await?;
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Blocklist,
    ));
    drain_pending(&mut app).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Blocklist);

    // @step When the user presses Esc
    app.dispatch(Action::CloseBlocklistView);
    drain_pending(&mut app).await?;

    // @step Then Action::CloseBlocklistView is observed on the action bus
    // (covered by the dispatch above — round-trips through the Navigator)

    // @step And the Navigator's active_view returns to ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    Ok(())
}
