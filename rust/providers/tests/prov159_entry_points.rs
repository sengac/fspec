//! Feature: spec/features/codex-refreshing-client.feature
//! Feature: spec/features/prov159-entry-points.feature

#![cfg(feature = "test-support")]

#[path = "prov159_support/network.rs"]
pub mod network;
pub mod prov159_support;

use codelet_providers::codex::auth_lifecycle::LoginAttempt;
use codelet_providers::codex::codex_auth::{
    clear_codex_oauth, get_codex_api_key_sync, read_codex_auth, refresh_codex_tokens,
};
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
use network::{bounded, RefreshServer};
use prov159_support::{tokens, Fixture, TestResult};
use serial_test::serial;
use wiremock::{matchers::body_string_contains, Mock, MockServer, ResponseTemplate};

#[derive(Clone, Copy, Debug)]
enum EntryPoint {
    Shared,
    Legacy,
}

#[derive(Clone, Copy)]
enum Transition {
    Logout,
    DifferentAccount,
    SameAccount,
}

fn seed(fixture: &Fixture, entry: EntryPoint, lifetime: i64) -> TestResult {
    fixture.seed("A")?;
    let path = fixture.directory.path().join("auth.json");
    let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    document["fspec_expires_at"] = (chrono::Utc::now().timestamp() + lifetime).into();
    if matches!(entry, EntryPoint::Legacy) {
        document
            .as_object_mut()
            .ok_or("fixture document is not an object")?
            .remove("OPENAI_API_KEY");
    }
    std::fs::write(path, document.to_string())?;
    Ok(())
}

async fn refresh(entry: EntryPoint) -> TestResult {
    match entry {
        EntryPoint::Shared => {
            refresh_codex_tokens("refresh-A").await?;
        }
        EntryPoint::Legacy => {
            tokio::task::spawn_blocking(get_codex_api_key_sync).await??;
        }
    }
    Ok(())
}

async fn fresh_credentials(entry: EntryPoint) -> TestResult {
    let fixture = Fixture::new()?;
    seed(&fixture, entry, 3600)?;
    let issuer = MockServer::start().await;
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    Mock::given(body_string_contains("subject_token=id-A"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "exchanged-api-key"
        })))
        .mount(&issuer)
        .await;
    Mock::given(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id_token": "id-A", "access_token": "unnecessary-refresh",
            "refresh_token": "unnecessary-rotation", "expires_in": 3600
        })))
        .mount(&issuer)
        .await;

    bounded(refresh(entry)).await??;

    let requests = issuer
        .received_requests()
        .await
        .ok_or("issuer recording disabled")?;
    assert!(
        requests
            .iter()
            .all(|request| !String::from_utf8_lossy(&request.body)
                .contains("grant_type=refresh_token")),
        "{entry:?} refreshed fresh disk credentials"
    );
    assert_eq!(
        requests.len(),
        usize::from(matches!(entry, EntryPoint::Legacy))
    );
    let stored = read_codex_auth()?
        .and_then(|auth| auth.tokens)
        .ok_or("tokens disappeared")?;
    assert_eq!(stored.refresh_token, "refresh-A");
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a14_shared_refresh_reuses_fresh_disk_credentials() -> TestResult {
    // @step Given fresh persisted credentials are available to shared refresh
    // @step When shared refresh is requested
    // @step Then existing tokens are reused without a refresh or exchange request
    fresh_credentials(EntryPoint::Shared).await
}

#[tokio::test]
#[serial]
async fn scenario_a14_legacy_exchange_reuses_fresh_disk_credentials() -> TestResult {
    // @step Given fresh persisted credentials are available to legacy API key exchange
    // @step When legacy API key exchange is requested
    // @step Then existing tokens are reused with only the API key exchange request
    fresh_credentials(EntryPoint::Legacy).await
}

async fn race(transition: Transition) -> TestResult {
    for entry in [EntryPoint::Shared, EntryPoint::Legacy] {
        let fixture = Fixture::new()?;
        seed(&fixture, entry, -60)?;
        let issuer = RefreshServer::start().await?;
        let _config = configure(OAuthTestConfig {
            issuer: issuer.uri.clone(),
            callback_address: "127.0.0.1:0".parse()?,
            state: "unused".into(),
        });
        let (result, transitioned) = bounded(async {
            tokio::join!(refresh(entry), async {
                bounded(issuer.started.notified()).await?;
                let transition_result = async {
                    match transition {
                        Transition::Logout => clear_codex_oauth()?,
                        Transition::DifferentAccount | Transition::SameAccount => {
                            let attempt = LoginAttempt::begin().await?;
                            let mut replacement =
                                tokens(if matches!(transition, Transition::SameAccount) {
                                    "A"
                                } else {
                                    "B"
                                });
                            replacement.access_token = "replacement-access".into();
                            replacement.refresh_token = "replacement-refresh".into();
                            attempt.commit(&replacement).await?;
                        }
                    }
                    Ok::<_, Box<dyn std::error::Error>>(())
                }
                .await;
                issuer.release.add_permits(2);
                transition_result
            })
        })
        .await?;
        transitioned?;

        assert!(
            result.is_err(),
            "{entry:?} accepted a refresh from a revoked login generation"
        );
        assert_eq!(
            issuer.requests()?.len(),
            1,
            "{entry:?} continued to token exchange after revocation"
        );
        let stored = read_codex_auth()?.ok_or("credential document disappeared")?;
        match transition {
            Transition::Logout => assert!(stored.tokens.is_none(), "{entry:?} resurrected logout"),
            Transition::DifferentAccount | Transition::SameAccount => {
                let stored = stored.tokens.ok_or("replacement login disappeared")?;
                assert_eq!(stored.access_token, "replacement-access");
                assert_eq!(stored.refresh_token, "replacement-refresh");
                assert_eq!(
                    stored.account_id,
                    if matches!(transition, Transition::SameAccount) {
                        "A"
                    } else {
                        "B"
                    }
                );
            }
        }
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
        assert_eq!(document["unrelated"]["keep"], true);
        if matches!(entry, EntryPoint::Shared) {
            assert_eq!(document["OPENAI_API_KEY"], "sk-independent");
        } else {
            assert!(
                document.get("OPENAI_API_KEY").is_none(),
                "legacy exchange cached a revoked API key"
            );
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a7_entry_points_cannot_resurrect_logout() -> TestResult {
    // @step Given shared and legacy entry points have acknowledged pending refreshes
    // @step When logout completes before the refresh responses are released
    // @step Then both entry points reject stale completion without restoring tokens or exchanging an API key
    race(Transition::Logout).await
}

#[tokio::test]
#[serial]
async fn scenario_a8_entry_points_cannot_overwrite_different_account() -> TestResult {
    // @step Given shared and legacy entry points have acknowledged pending refreshes
    // @step When a different account login completes before the refresh responses are released
    // @step Then both entry points reject stale completion and preserve the replacement credentials without API key exchange
    race(Transition::DifferentAccount).await
}

#[tokio::test]
#[serial]
async fn scenario_a9_entry_points_cannot_overwrite_same_account_login() -> TestResult {
    // @step Given shared and legacy entry points have acknowledged pending refreshes
    // @step When a same-account replacement login completes before the refresh responses are released
    // @step Then both entry points reject stale completion and preserve the replacement credentials without API key exchange
    race(Transition::SameAccount).await
}
