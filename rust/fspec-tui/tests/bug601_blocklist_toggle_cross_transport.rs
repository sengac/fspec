//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! BUG-601: turning a blocklist rule off or on for a session, and reading the
//! session's turned-off rules back, go through the same daemon over the
//! embedded and the WebSocket transport. Both backends talk to one
//! StubSessionManagerHandle, which keeps each session's turned-off rules.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{BlocklistRuleInfo, SessionId};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_embedded_and_websocket_transports_turn_a_rule_off_and_read_it_back_the_same_way(
) -> TestResult {
    // @step Given a daemon listing rule "git-checkout-block" behind an embedded and a WebSocket backend
    let temp = tempfile::tempdir().or_fail("tempdir")?;
    std::fs::create_dir_all(temp.path().join("spec")).or_fail("mkdir spec")?;
    std::fs::write(
        temp.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("work-units.json")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(temp.path()).or_fail("watcher")?);
    let stub = Arc::new(StubSessionManagerHandle::new());
    stub.seed_blocklist_rules(vec![BlocklistRuleInfo {
        id: "git-checkout-block".to_string(),
        pattern: "^git\\s+checkout\\b".to_string(),
        action: "block".to_string(),
        reason: "git checkout can overwrite uncommitted changes.".to_string(),
        guidance: None,
        source: "system".to_string(),
    }]);
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, handle)
            .with_cwd(temp.path().to_path_buf()),
    );
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );
    let s1 = SessionId::new("s-1");

    // @step When the WebSocket backend turns "git-checkout-block" off for "s-1"
    let answer = websocket
        .blocklist_set_rule_enabled(s1.clone(), "git-checkout-block".to_string(), false)
        .await
        .or_fail("websocket blocklist_set_rule_enabled")?;
    assert_eq!(answer, vec!["git-checkout-block".to_string()]);

    // @step Then the embedded backend reads "git-checkout-block" as turned off for "s-1"
    assert_eq!(
        embedded
            .blocklist_disabled_rules(s1.clone())
            .await
            .or_fail("embedded blocklist_disabled_rules")?,
        vec!["git-checkout-block".to_string()]
    );

    // @step When the embedded backend turns "git-checkout-block" on for "s-1"
    let answer = embedded
        .blocklist_set_rule_enabled(s1.clone(), "git-checkout-block".to_string(), true)
        .await
        .or_fail("embedded blocklist_set_rule_enabled")?;
    assert_eq!(answer, Vec::<String>::new());

    // @step Then the WebSocket backend reads no rule turned off for "s-1"
    assert_eq!(
        websocket
            .blocklist_disabled_rules(s1.clone())
            .await
            .or_fail("websocket blocklist_disabled_rules")?,
        Vec::<String>::new()
    );
    let refused = websocket
        .blocklist_set_rule_enabled(s1, "no-such-rule".to_string(), false)
        .await
        .err()
        .map(|error| error.to_string());
    assert_eq!(
        refused.as_deref(),
        Some("the blocklist has no rule \"no-such-rule\"")
    );
    Ok(())
}
