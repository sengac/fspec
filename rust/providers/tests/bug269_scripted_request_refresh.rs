//! Feature: spec/features/scripted-oauth-request-refresh.feature
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/bug269_script.rs"]
mod script;
#[path = "support/bug269.rs"]
mod support;

use environment::{Environment, TestResult};
use serde_json::{json, Value};
use std::time::Duration;
use support::{NAME, NEW, OLD, REFRESH};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn every_real_completion_front_door_refreshes_before_authorization() -> TestResult {
    // @step Given a selected scripted OAuth provider has expired synthetic tokens and a numeric loopback issuer
    let environment = Environment::new()?;
    for source in [script::SCRIPT.to_owned(), script::legacy()] {
        for door in 0..4 {
            support::seed(true)?;
            let server = support::server().await?;
            support::issuer(&server, 200, Duration::ZERO).await;
            let provider =
                support::provider(environment.directory.path(), &server, &source, false)?;
            // @step When each streaming and nonstreaming front door completes with preferred or legacy refresh hooks
            support::complete(provider, door).await?;
            // @step Then the local issuer receives the old refresh token once and inference receives only fresh authorization
            let requests = support::requests(&server, "/token").await?;
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].body_json::<Value>()?["refresh_token"], REFRESH);
            support::assert_inference(&server, 1, NEW).await?;
            // @step And the new tokens are persisted with the ordinary completion and usage intact
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(support::token_path())?)?,
                support::tokens(false)
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn independent_concurrent_providers_coalesce_one_credential_refresh() -> TestResult {
    // @step Given independently constructed scripted providers share one expired temporary credential store
    let environment = Environment::new()?;
    support::seed(true)?;
    let server = support::server().await?;
    support::issuer(&server, 200, Duration::from_millis(150)).await;
    let first = support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
    let second = support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
    // @step When their real completion requests run concurrently
    let (first, second) = tokio::join!(support::complete(first, 0), support::complete(second, 3));
    first?;
    second?;
    // @step Then one issuer exchange precedes all freshly authorized inference requests
    assert_eq!(support::requests(&server, "/token").await?.len(), 1);
    support::assert_inference(&server, 2, NEW).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn existing_script_authentication_ownership_remains_unchanged() -> TestResult {
    // @step Given fresh missing tokens missing hooks or headers owned Custom authentication
    let environment = Environment::new()?;
    for mode in ["fresh", "missing", "hooks", "custom", "bearer", "api_key"] {
        let server = support::server().await?;
        support::seed(mode != "fresh")?;
        if mode == "missing" {
            std::fs::remove_file(support::token_path())?;
        }
        let source = if mode == "hooks" {
            script::SCRIPT
                .split("fn auth_needs_refresh")
                .next()
                .ok_or("missing script prefix")?
                .to_owned()
        } else if mode == "custom" {
            script::headers_owned()
        } else {
            script::SCRIPT.to_owned()
        };
        let owned_path = support::token_path();
        Mock::given(method("POST"))
            .and(path("/owned"))
            .respond_with(move |_: &wiremock::Request| {
                let outcome = serde_json::to_vec(&support::tokens(false))
                    .map_err(std::io::Error::other)
                    .and_then(|bytes| std::fs::write(&owned_path, bytes));
                ResponseTemplate::new(if outcome.is_ok() { 200 } else { 500 })
                    .set_body_json(json!({"settled":true}))
            })
            .mount(&server)
            .await;
        let provider =
            script::owned_provider(environment.directory.path(), &server, &source, mode)?;
        // @step When actual custom completion requests are sent
        support::complete(provider, 0).await?;
        // @step Then automatic refresh does not override existing ownership or duplicate a headers owned exchange
        assert!(support::requests(&server, "/token").await?.is_empty());
        assert_eq!(
            support::requests(&server, "/owned").await?.len(),
            usize::from(mode == "custom")
        );
        support::assert_inference(
            &server,
            1,
            match mode {
                "missing" => "synthetic-missing",
                "hooks" | "bearer" | "api_key" => OLD,
                _ => NEW,
            },
        )
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn project_selected_scripts_remain_authoritative_during_refresh() -> TestResult {
    use codelet_providers::custom::CustomProvider;
    // @step Given project and global provider scripts disagree about refresh behavior
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let global = root.join("global/credentials");
    std::env::set_var("FSPEC_HOME", &global);
    support::seed(true)?;
    let server = support::server().await?;
    support::issuer(&server, 200, Duration::ZERO).await;
    let config = support::config(root, &server, script::SCRIPT, false)?;
    support::install_project_and_global(root, &config)?;
    // @step When a normally selected project provider issues a real completion request
    let agent = CustomProvider::create_rig_agent(
        root,
        NAME,
        "local",
        uuid::Uuid::new_v4(),
        None,
        None,
        false,
    )?;
    support::complete(agent.agent().model.provider().clone(), 2).await?;
    // @step Then its selected compiled script and OAuth config alone determine issuer headers and inference
    assert_eq!(support::requests(&server, "/token").await?.len(), 1);
    support::assert_inference(&server, 1, NEW).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_refreshes_retain_credentials_and_redact_diagnostics() -> TestResult {
    // @step Given a local issuer rejects refresh or fails transiently for expired synthetic credentials
    let environment = Environment::new()?;
    for status in [403, 503, 0] {
        support::seed(true)?;
        let server = support::server().await?;
        support::issuer(&server, status, Duration::ZERO).await;
        let provider = support::failure_provider(environment.directory.path(), &server, status)?;
        // @step When an actual completion request reaches refresh
        let error = support::complete(provider.clone(), 3)
            .await
            .err()
            .ok_or("refresh unexpectedly succeeded")?;
        // @step Then no inference is sent no token changes and no credential or issuer body appears in diagnostics
        for secret in [OLD, NEW, REFRESH] {
            assert!(!error.contains(secret), "{error}");
        }
        assert!(!error.contains("access_token"), "{error}");
        assert!(support::requests(&server, "/inference").await?.is_empty());
        assert_eq!(
            support::requests(&server, "/token").await?.len(),
            usize::from(status != 0)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(support::token_path())?)?,
            support::tokens(true)
        );
        // @step And a later explicit request can refresh successfully without automatic inference replay
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(support::tokens(false)))
            .with_priority(1)
            .mount(&server)
            .await;
        let retry =
            support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
        support::complete(retry, 3).await?;
        assert_eq!(
            support::requests(&server, "/token").await?.len(),
            1 + usize::from(status != 0)
        );
        support::assert_inference(&server, 1, NEW).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn caller_cancellation_and_deadline_prevent_inference_after_admitted_refresh() -> TestResult {
    // @step Given a synchronous local refresh has been observed at the issuer
    let environment = Environment::new()?;
    for deadline in [false, true] {
        support::seed(true)?;
        let server = support::server().await?;
        support::issuer(&server, 200, Duration::from_millis(300)).await;
        let provider =
            support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
        let (expire, expired) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            if deadline {
                tokio::select! { result=support::complete(provider,0)=>result, _=support::caller_deadline(expired)=>Err("caller deadline".into()) }
            } else {
                support::complete(provider, 0).await
            }
        });
        support::wait_requests(&server, "/token", 1).await?;
        // @step When the actual requesting task is cancelled or its caller deadline expires
        if deadline {
            expire
                .send(tokio::time::Instant::now() + Duration::from_millis(20))
                .map_err(|_| "deadline receiver gone")?;
            assert_eq!(task.await?, Err("caller deadline".into()));
        } else {
            task.abort();
            assert!(task.await.err().is_some_and(|error| error.is_cancelled()));
        }
        // @step Then its authorized refresh may settle and persist but sends no inference
        support::wait_persisted(&support::token_path()).await?;
        assert!(support::requests(&server, "/inference").await?.is_empty());
        // @step And all admitted operations settle before the isolated fixture environment is restored
        let provider =
            support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
        support::complete(provider, 0).await?;
        assert_eq!(support::requests(&server, "/token").await?.len(), 1);
        support::assert_inference(&server, 1, NEW).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn credential_root_changes_fail_closed_across_refresh_waits() -> TestResult {
    // @step Given an admitted refresh and another waiting request share a temporary credential path
    let environment = Environment::new()?;
    support::seed(true)?;
    let original_path = support::token_path();
    let server = support::server().await?;
    support::issuer(&server, 200, Duration::from_millis(300)).await;
    let provider = support::provider(environment.directory.path(), &server, script::SCRIPT, false)?;
    let first_provider = provider.clone();
    let first = tokio::spawn(support::complete(first_provider, 0));
    support::wait_requests(&server, "/token", 1).await?;
    let second = support::complete(provider, 0);
    tokio::pin!(second);
    assert!(tokio::time::timeout(Duration::from_millis(30), &mut second)
        .await
        .is_err());
    // @step When the active credential root changes before refresh persistence or waiter admission
    std::env::set_var(
        "FSPEC_HOME",
        environment.directory.path().join("replacement"),
    );
    support::seed(false)?;
    let replacement_path = support::token_path();
    let before = std::fs::read(&replacement_path)?;
    // @step Then neither operation reads or writes the replacement credentials or sends inference
    assert!(first.await?.is_err());
    assert!(second.await.is_err());
    assert_eq!(std::fs::read(replacement_path)?, before);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(original_path)?)?,
        support::tokens(true)
    );
    assert_eq!(support::requests(&server, "/token").await?.len(), 1);
    assert!(support::requests(&server, "/inference").await?.is_empty());
    Ok(())
}
