//! Feature: spec/features/copilot-transmission-origin.feature
#![cfg(feature = "test-support")]
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/bug268_lineage.rs"]
pub mod support;

use codelet_providers::{
    copilot::{
        test_support::TestIssuerGuard, CopilotAuthJson, CopilotDeploymentType, CopilotProvider,
    },
    session::ProviderSession,
    ProviderAdapter,
};
use environment::{Environment, TestResult};
use rig::client::CompletionClient;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
#[serial_test::serial]
async fn origin_changes_during_actual_transmission_invalidate_replay_before_sending() -> TestResult
{
    // @step Given a live Copilot session owns opaque before its next request refreshes
    let environment = Environment::new()?;
    let former = support::server().await;
    let current = support::server().await;
    let session = ProviderSession::for_provider("github-copilot");
    let history = support::complete(&support::model(&former, &session)?, vec![], "seed").await?;
    let issuer = wiremock::MockServer::start().await;
    let exchanges = Arc::new(AtomicUsize::new(0));
    let observed = exchanges.clone();
    let old_origin = former.uri();
    let new_origin = current.uri();
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(move |_: &wiremock::Request| {
            let first = observed.fetch_add(1, Ordering::SeqCst) == 0;
            wiremock::ResponseTemplate::new(200).set_body_json(json!({
                "token":if first {"synthetic-instant-expiry"} else {"synthetic-renewed-new-origin"},
                "expires_at":if first {0} else {u64::MAX},
                "endpoints":{"api":if first {old_origin.clone()} else {new_origin.clone()}}
            }))
        })
        .expect(2)
        .mount(&issuer)
        .await;
    let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    let provider = CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        CopilotAuthJson {
            github_oauth_token: "synthetic-logical-account".into(),
            copilot_token: Some("synthetic-expired".into()),
            copilot_token_expires_at: Some(0),
            endpoints_api: Some(former.uri()),
            enterprise_url: None,
        },
        "gpt-5",
    )?
    .with_session(&session);
    let model = provider.client().completion_model("gpt-5");
    // @step When the existing refresh coordinator changes the API origin after model preparation but before transmission
    let history = support::complete(&model, history, "new").await?;
    // @step Then the new origin receives fresh authorization without the former origin opaque
    assert!(support::last_request(&current)
        .await?
        .get("reasoning_opaque")
        .is_none());
    let requests = current.received_requests().await.ok_or("missing capture")?;
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer synthetic-renewed-new-origin"
    );
    assert_eq!(
        former
            .received_requests()
            .await
            .ok_or("missing former capture")?
            .len(),
        1
    );
    // @step And a compatible follow up retains only opaque settled at the new origin
    support::complete(&support::model(&current, &session)?, history, "probe").await?;
    assert_eq!(
        support::last_request(&current).await?["reasoning_opaque"],
        "synthetic-opaque-new"
    );
    assert_eq!(exchanges.load(Ordering::SeqCst), 2);
    Ok(())
}
