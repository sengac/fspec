//! Feature: spec/features/codex-oauth-login.feature
//! Feature: spec/features/prov159-lifecycle-regressions.feature

#[path = "prov159_support/network.rs"]
pub mod network;
pub mod prov159_support;

use codelet_providers::codex::auth_lifecycle::{cancel_login, LoginAttempt};
use codelet_providers::codex::codex_auth::{delete_codex_auth, read_codex_auth, write_codex_auth};
use prov159_support::{auth, tokens, Fixture, TestResult};
use serial_test::serial;

#[tokio::test]
async fn external_auth_transition() -> TestResult {
    let Ok(operation) = std::env::var("PROV159_AUTH_TRANSITION") else {
        return Ok(());
    };
    keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    match operation.as_str() {
        "delete" => delete_codex_auth()?,
        "login" => write_codex_auth(&auth("new"))?,
        "raw" => {
            let home = std::path::PathBuf::from(std::env::var_os("CODEX_HOME").ok_or("no home")?);
            std::fs::write(home.join("auth.json"), serde_json::to_vec(&auth("new"))?)?;
        }
        "begin" => {
            let _attempt = LoginAttempt::begin().await?;
        }
        "lock" => {
            let home = std::path::PathBuf::from(std::env::var_os("CODEX_HOME").ok_or("no home")?);
            let guard = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(home.join(".fspec-auth.lock"))?;
            guard.lock()?;
            std::fs::write(home.join("lock-ready"), "ready")?;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        _ => return Err("unknown transition".into()),
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a10_external_transition_fences_pending_login() -> TestResult {
    // @step Given a pending login with or without previously persisted credentials
    // @step When another process deletes credentials, logs in, begins login or replaces the credential document
    // @step Then the pending login cannot overwrite the authoritative external state
    for seeded in [false, true] {
        for operation in ["delete", "login", "begin", "raw"] {
            let fixture = Fixture::new()?;
            if seeded {
                fixture.seed("old")?;
            }
            let attempt = LoginAttempt::begin().await?;
            let status = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", "external_auth_transition", "--nocapture"])
                .env("PROV159_AUTH_TRANSITION", operation)
                .status()?;
            assert!(status.success(), "external credential transition failed");
            assert!(
                attempt.commit(&tokens("stale")).await.is_err(),
                "pending login overwrote external {operation} (seeded={seeded})"
            );
            let stored = read_codex_auth()?.and_then(|auth| auth.tokens);
            if operation == "login" || operation == "raw" {
                assert_eq!(stored.ok_or("replacement login lost")?.account_id, "new");
            } else if operation == "begin" && seeded {
                assert_eq!(stored.ok_or("old login lost")?.account_id, "old");
            } else {
                assert!(stored.is_none(), "login resurrected external logout");
            }
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a10_equivalent_home_paths_share_cancellation() -> TestResult {
    // @step Given a pending login associated with a credential home
    // @step When login is cancelled through an equivalent filesystem path
    // @step Then the original attempt cannot persist credentials
    let fixture = Fixture::new()?;
    let attempt = LoginAttempt::begin().await?;
    let alias = fixture.directory.path().join("alias");
    std::fs::create_dir(&alias)?;
    std::env::set_var("CODEX_HOME", alias.join(".."));
    cancel_login();
    assert!(
        attempt.commit(&tokens("stale")).await.is_err(),
        "path alias bypassed cancellation"
    );
    Ok(())
}

#[cfg(feature = "test-support")]
#[tokio::test]
#[serial]
async fn scenario_a10_stale_device_completion_preserves_replacement() -> TestResult {
    // @step Given a new device session replaces an older session
    // @step When completion is attempted for the old session and then the replacement
    // @step Then the old session fails without preventing the replacement from reaching the issuer
    use codelet_providers::codex::codex_device_auth::{complete_device_login, start_device_login};
    use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

    let _fixture = Fixture::new()?;
    let issuer = MockServer::start().await;
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    for device in ["old", "new"] {
        issuer.reset().await;
        Mock::given(path("/api/accounts/deviceauth/usercode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_auth_id": device, "user_code": "ABCD", "interval": 1
            })))
            .mount(&issuer)
            .await;
        start_device_login().await?;
    }
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "access_denied"
        })))
        .expect(1)
        .mount(&issuer)
        .await;
    assert!(complete_device_login("old").await.is_err());
    let error = complete_device_login("new")
        .await
        .err()
        .ok_or("denial unexpectedly succeeded")?;
    assert!(
        error.to_string().contains("access_denied"),
        "stale completion removed replacement: {error}"
    );
    issuer.verify().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a10_deadline_while_awaiting_persistence_cannot_commit() -> TestResult {
    // @step Given browser authorization has reached token exchange and another process holds the storage lock
    // @step When the login deadline expires while persistence waits for that lock
    // @step Then login fails and no credentials appear after the lock is released
    use base64::Engine;
    use codelet_providers::codex::codex_oauth_server::{
        browser_oauth_login_inner, OAuthServerConfig,
    };
    use network::{bounded, RefreshServer};
    use tokio::net::TcpListener;

    let fixture = Fixture::new()?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"chatgpt_account_id":"stale"}"#);
    let issuer = RefreshServer::with_body(
        serde_json::json!({
            "id_token": format!("e30.{payload}.signature"), "access_token": "access",
            "refresh_token": "refresh", "expires_in": 3600
        })
        .to_string(),
    )
    .await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let pending = tokio::spawn(browser_oauth_login_inner(OAuthServerConfig {
        listener,
        issuer_url: issuer.uri.clone(),
        open_browser: false,
        timeout_ms: 200,
        pkce: None,
        state: Some("state".into()),
    }));
    reqwest::get(format!(
        "http://{address}/auth/callback?code=code&state=state"
    ))
    .await?;
    bounded(issuer.started.notified()).await?;
    let mut locker = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "external_auth_transition", "--nocapture"])
        .env("PROV159_AUTH_TRANSITION", "lock")
        .spawn()?;
    bounded(async {
        while !fixture.directory.path().join("lock-ready").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await?;
    issuer.release.add_permits(1);
    let result = bounded(pending).await??;
    assert!(locker.wait()?.success());
    assert!(
        result.is_err(),
        "blocking storage lock bypassed the OAuth deadline"
    );
    assert!(
        read_codex_auth()?.and_then(|auth| auth.tokens).is_none(),
        "expired login persisted after the storage lock was released"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a10_cancellation_while_awaiting_persistence_cannot_commit() -> TestResult {
    // @step Given a login commit waits for a storage lock held by another process
    // @step When the pending login is cancelled
    // @step Then commit promptly fails and no credentials appear after the lock is released
    let fixture = Fixture::new()?;
    let attempt = LoginAttempt::begin().await?;
    let mut locker = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "external_auth_transition", "--nocapture"])
        .env("PROV159_AUTH_TRANSITION", "lock")
        .spawn()?;
    network::bounded(async {
        while !fixture.directory.path().join("lock-ready").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await?;
    let pending = tokio::spawn(async move { attempt.commit(&tokens("stale")).await });
    tokio::task::yield_now().await;
    cancel_login();
    let cancelled = tokio::time::timeout(std::time::Duration::from_millis(100), pending).await;
    assert!(locker.wait()?.success());
    assert!(
        cancelled??.is_err(),
        "cancelled storage wait reported successful login"
    );
    assert!(
        read_codex_auth()?.and_then(|auth| auth.tokens).is_none(),
        "cancelled login persisted after storage became available"
    );
    Ok(())
}
