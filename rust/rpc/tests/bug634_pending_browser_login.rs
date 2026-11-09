//! Feature: spec/features/codex-pending-browser-login-invalidation.feature

use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_test_helpers::{setup_temp_directory, OrFail, TestResult};
use wiremock::MockServer;

#[path = "../../providers/tests/fixtures/mod.rs"]
mod fixtures;

fn service(root: &std::path::Path) -> Result<FspecServiceImpl, codelet_test_helpers::TestError> {
    std::fs::create_dir(root.join("spec"))?;
    std::fs::write(root.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    Ok(FspecServiceImpl::new(Arc::new(SharedFspecService::new(
        Arc::new(WorkUnitsWatcher::new(root)?),
    ))))
}

#[tokio::test]
#[serial_test::serial]
async fn logout_invalidates_pending_authorization() -> TestResult {
    // @step Given a daemon with isolated Codex credentials and a local token issuer
    let root = setup_temp_directory()?;
    let (_home, _home_guard) = fixtures::setup_codex_home()?;
    let issuer = MockServer::start().await;
    let _issuer_guard = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    let daemon = service(root.path())?;
    // @step When browser authorization starts and I log out before the callback
    let start = daemon
        .clone()
        .oauth_headless_start(tarpc::context::current(), "codex".into())
        .await?;
    daemon
        .clone()
        .oauth_clear_tokens(tarpc::context::current(), "codex".into())
        .await?;
    // @step Then completing that authorization is rejected without token exchange
    assert!(daemon
        .oauth_headless_complete(
            tarpc::context::current(),
            "codex".into(),
            format!("late-code#{}", start.pkce_verifier),
            start.pkce_verifier
        )
        .await
        .is_err());
    assert!(issuer
        .received_requests()
        .await
        .or_fail("recorded requests")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn newer_authorization_invalidates_older_attempt() -> TestResult {
    // @step Given a daemon with isolated Codex credentials and a local token issuer
    let root = setup_temp_directory()?;
    let (_home, _home_guard) = fixtures::setup_codex_home()?;
    let issuer = MockServer::start().await;
    let _issuer_guard = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    let daemon = service(root.path())?;
    // @step When I start browser authorization twice
    let start = daemon
        .clone()
        .oauth_headless_start(tarpc::context::current(), "codex".into())
        .await?;
    let next = codelet_test_helpers::deadlock_guard(
        "new browser attempt supersedes old",
        daemon
            .clone()
            .oauth_headless_start(tarpc::context::current(), "codex".into()),
    )
    .await??;
    assert_ne!(next.pkce_verifier, start.pkce_verifier);
    // @step Then the older authorization is rejected without token exchange
    assert!(daemon
        .clone()
        .oauth_headless_complete(
            tarpc::context::current(),
            "codex".into(),
            format!("late-code#{}", start.pkce_verifier),
            start.pkce_verifier
        )
        .await
        .is_err());
    assert!(issuer
        .received_requests()
        .await
        .or_fail("recorded requests")?
        .is_empty());
    daemon
        .oauth_clear_tokens(tarpc::context::current(), "codex".into())
        .await?;
    Ok(())
}
