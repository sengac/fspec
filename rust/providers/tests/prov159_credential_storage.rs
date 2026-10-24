//! Feature: spec/features/codex-oauth-login.feature
//! Primary feature: spec/features/prov159-credential-storage.feature

mod prov159_support;

#[cfg(target_os = "macos")]
#[path = "prov159_credential_storage/keyring.rs"]
mod storage_keyring;

use codelet_providers::codex::codex_auth::{delete_codex_auth, read_codex_auth, write_codex_auth};
#[cfg(target_os = "macos")]
use keyring::credential::CredentialApi;
use prov159_support::{auth, Fixture, TestResult};
use serial_test::serial;

#[test]
#[serial]
fn scenario_a2_login_replaces_the_preferred_account_consistently() -> TestResult {
    // @step Given Codex account A exists in the preferred keychain and file fallback
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    #[cfg(target_os = "macos")]
    fixture
        .keychain()
        .set_password(&serde_json::to_string(&auth("A"))?)?;

    let path = fixture.directory.path().join("auth.json");
    let mut previous: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    previous["fspec_expires_at"] = (chrono::Utc::now().timestamp() + 7200).into();
    std::fs::write(&path, previous.to_string())?;
    // @step When the user completes browser or device login as account B
    write_codex_auth(&auth("B"))?;

    // @step Then the settings UI, a newly constructed provider and a restarted application select account B
    // @step And no preferred or fallback source restores account A
    let stored = read_codex_auth()?.ok_or("login must persist credentials")?;
    assert_eq!(stored.tokens.ok_or("login tokens missing")?.account_id, "B");
    #[cfg(target_os = "macos")]
    match fixture.keychain().get_password() {
        Ok(document) => assert!(!document.contains("access-A")),
        Err(keyring::Error::NoEntry) => (),
        Err(error) => return Err(error.into()),
    }

    // @step And independent API keys and unrelated credential fields are preserved
    assert_eq!(stored.openai_api_key.as_deref(), Some("sk-independent"));
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    assert_eq!(document["unrelated"]["keep"], true);
    assert!(
        document["fspec_expires_at"].is_null(),
        "new login inherited the previous account's expiry"
    );
    #[cfg(target_os = "macos")]
    {
        fixture
            .keychain()
            .set_password(&serde_json::to_string(&auth("B"))?)?;
        let before = std::fs::read(&path)?;
        let _client = codelet_providers::codex::refreshing_client::RefreshingCodexClient::new_oauth(
            "access-B".into(),
            "refresh-B".into(),
            "B".into(),
            Some(3600),
            "http://127.0.0.1:1".into(),
        );
        assert_eq!(
            std::fs::read(&path)?,
            before,
            "matching preferred tokens discarded the active generation"
        );
    }
    #[cfg(target_os = "macos")]
    for fallback in ["{", "[]", r#"{"tokens":true,"OPENAI_API_KEY":false}"#] {
        fixture
            .keychain()
            .set_password(&serde_json::to_string(&auth("preferred"))?)?;
        std::fs::write(&path, fallback)?;
        let stored = read_codex_auth()?.ok_or("preferred credentials must remain readable")?;
        assert_eq!(
            stored.tokens.ok_or("preferred tokens missing")?.account_id,
            "preferred"
        );
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_a3_provider_deletion_clears_all_oauth_sources() -> TestResult {
    // @step Given the Codex provider uses OAuth account A from the keychain with a file fallback
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    #[cfg(target_os = "macos")]
    fixture
        .keychain()
        .set_password(&serde_json::to_string(&auth("A"))?)?;

    // @step When the user deletes the Codex provider and credentials are detected again after restarting
    delete_codex_auth()?;
    delete_codex_auth()?;

    // @step Then neither the keychain nor the file fallback restores the deleted OAuth account
    assert!(read_codex_auth()?
        .and_then(|stored| stored.tokens)
        .is_none());
    Ok(())
}

#[test]
#[serial]
fn scenario_a15_persistence_is_private_complete_and_failure_safe() -> TestResult {
    // @step And credential replacement respects concurrent logout and newer login transitions
    // @step Given parallel readers and lifecycle writers share an existing Codex credential document with independent API keys and unrelated fields
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    // @step When credentials are replaced and a persistence failure is injected before replacement completes
    write_codex_auth(&auth("B"))?;
    let path = fixture.directory.path().join("auth.json");
    // @step Then every successful read observes a complete valid credential document
    let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    // @step And successful updates preserve independent API keys and unrelated fields
    assert_eq!(document["OPENAI_API_KEY"], "sk-independent");
    assert_eq!(document["unrelated"]["keep"], true);
    let barrier = std::sync::Barrier::new(4);
    let readers = std::thread::scope(|scope| {
        let readers: Vec<_> = (0..3)
            .map(|_| {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    (0..100).all(|_| {
                        std::fs::read(path)
                            .ok()
                            .and_then(|bytes| {
                                serde_json::from_slice::<serde_json::Value>(&bytes).ok()
                            })
                            .is_some_and(|document| document["unrelated"]["keep"] == true)
                    })
                })
            })
            .collect();
        barrier.wait();
        for _ in 0..50 {
            write_codex_auth(&auth("B"))?;
        }
        Ok::<_, Box<dyn std::error::Error>>(
            readers
                .into_iter()
                .all(|reader| reader.join().unwrap_or(false)),
        )
    })?;
    assert!(readers, "parallel readers must only see complete documents");
    // @step And new credential files have Unix mode 0600 and replacement does not expose credentials through temporary files
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o600
        );
    }
    // @step And the failed replacement reports an error and leaves the prior valid document intact
    let before = std::fs::read(&path)?;
    std::env::set_var("CODEX_HOME", &path);
    assert!(write_codex_auth(&auth("C")).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
#[serial]
fn scenario_a16_keyring_failures_are_not_hidden_account_fallback() -> TestResult {
    // @step And the settings UI does not claim successful logout or login while the storage operation has failed
    // @step And partial cleanup failures identify the incomplete operation without exposing credential contents
    // @step Given the preferred Codex keyring contains account A and the fallback file contains account B
    let fixture = Fixture::new()?;
    fixture.seed("B")?;
    let keychain = fixture.keychain();
    keychain.set_password(&serde_json::to_string(&auth("A"))?)?;

    // @step When a keyring read, login save, OAuth clear or provider deletion fails
    keychain.set_error(keyring::Error::NoStorageAccess(Box::new(
        std::io::Error::other("locked"),
    )));
    // @step Then the operation reports a clear storage failure rather than success
    // @step And a keyring failure is not treated as missing credentials or permission to select fallback account B
    assert!(
        read_codex_auth().is_err(),
        "a locked keychain must not select file account B"
    );
    keychain.set_error(keyring::Error::NoStorageAccess(Box::new(
        std::io::Error::other("locked"),
    )));
    assert!(write_codex_auth(&auth("C")).is_err());
    keychain.set_error(keyring::Error::NoStorageAccess(Box::new(
        std::io::Error::other("locked"),
    )));
    assert!(delete_codex_auth().is_err());
    storage_keyring::verify_failure_safety(&fixture)?;
    Ok(())
}
