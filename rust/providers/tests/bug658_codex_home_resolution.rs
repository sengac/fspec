//! BUG-658 — the Codex home is resolved without falling back, and sign-out,
//! delete and cancel cancel pending logins only once the home exists, under
//! the auth lock, before the credential change.
//!
//! Feature: spec/features/codex-home-is-resolved-without-falling-back.feature
//!
//! Every test is deterministic: the login attempts are begun, cancelled and
//! committed in a fixed order on one task, with no timing assumptions.

pub mod prov159_support;

use codelet_providers::codex::auth_lifecycle::{cancel_login, LoginAttempt};
use codelet_providers::codex::codex_auth::{
    clear_codex_oauth, delete_codex_auth, read_codex_auth, AuthError,
};
use codelet_providers::codex::codex_device_auth::complete_device_login;
use codelet_test_helpers::{deadlock_guard, deadlock_guard_blocking};
use prov159_support::{tokens, Fixture, TestResult};
use serial_test::serial;

/// Points CODEX_HOME at a directory inside the fixture that does not exist
/// (the fixture restores CODEX_HOME when dropped).
fn missing_home(fixture: &Fixture) -> std::path::PathBuf {
    let home = fixture.directory.path().join("missing-codex-home");
    std::env::set_var("CODEX_HOME", &home);
    home
}

fn stored_tokens() -> Result<bool, AuthError> {
    Ok(read_codex_auth()?.and_then(|auth| auth.tokens).is_some())
}

#[tokio::test]
#[serial]
async fn an_unresolvable_codex_home_is_an_error_naming_the_path() -> TestResult {
    // @step Given CODEX_HOME points at a directory that does not exist
    let fixture = Fixture::new()?;
    let home = missing_home(&fixture);

    // @step When a device login is completed
    let error = complete_device_login("device")
        .await
        .err()
        .ok_or("device login completed against a missing Codex home")?;

    // @step Then it fails with an error that names the Codex home path
    let message = error.to_string();
    assert!(
        message.contains(&home.display().to_string()),
        "error does not name the Codex home {}: {message}",
        home.display()
    );
    // @step And it is not reported as a changed login
    assert!(
        !matches!(error, AuthError::Changed),
        "a missing Codex home was reported as a changed login"
    );
    assert!(!home.exists(), "completing a login created the Codex home");
    Ok(())
}

#[tokio::test]
#[serial]
async fn sign_out_delete_and_cancel_work_when_there_is_no_codex_home() -> TestResult {
    // @step Given CODEX_HOME points at a directory that does not exist
    let fixture = Fixture::new()?;
    let home = missing_home(&fixture);

    // @step When the Codex OAuth tokens are cleared, the credentials are deleted and the login is cancelled
    // @step Then each succeeds
    // @step And the Codex home now exists
    for (operation, run) in [
        (
            "clear_codex_oauth",
            clear_codex_oauth as fn() -> Result<(), AuthError>,
        ),
        ("delete_codex_auth", delete_codex_auth),
        ("cancel_login", cancel_login),
    ] {
        if home.exists() {
            std::fs::remove_dir_all(&home)?;
        }
        run().map_err(|error| format!("{operation} failed without a Codex home: {error}"))?;
        assert!(
            home.is_dir(),
            "{operation} did not leave the Codex home in place"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_login_begun_before_sign_out_cannot_commit_after_it() -> TestResult {
    // @step Given a Codex login attempt begun before sign-out
    let _fixture = Fixture::new()?;
    let stale = LoginAttempt::begin().await?;

    // @step When the user signs out and the attempt then commits its tokens
    clear_codex_oauth()?;
    let committed = stale.commit(&tokens("stale")).await;

    // @step Then the commit fails and no tokens are stored
    assert!(
        matches!(committed, Err(AuthError::Changed)),
        "a login begun before sign-out committed after it: {committed:?}"
    );
    assert!(!stored_tokens()?, "sign-out was undone by a stale login");
    drop(stale);

    // @step And a login begun after the sign-out commits its tokens
    let fresh = LoginAttempt::begin().await?;
    fresh.commit(&tokens("fresh")).await?;
    let stored = read_codex_auth()?
        .and_then(|auth| auth.tokens)
        .ok_or("the login begun after sign-out stored no tokens")?;
    assert_eq!(stored.account_id, "fresh");
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_login_begun_before_delete_cannot_commit_after_it() -> TestResult {
    // @step Given a Codex login attempt begun before the credentials are deleted
    let fixture = Fixture::new()?;
    fixture.seed("old")?;
    let stale = LoginAttempt::begin().await?;

    // @step When the credentials are deleted and the attempt then commits its tokens
    delete_codex_auth()?;
    let committed = stale.commit(&tokens("stale")).await;

    // @step Then the commit fails and no tokens are stored
    assert!(
        matches!(committed, Err(AuthError::Changed)),
        "a login begun before delete committed after it: {committed:?}"
    );
    assert!(!stored_tokens()?, "delete was undone by a stale login");
    Ok(())
}

#[tokio::test]
#[serial]
async fn cancelling_a_login_does_not_wait_for_the_auth_lock() -> TestResult {
    // @step Given a Codex login attempt
    let fixture = Fixture::new()?;
    let attempt = LoginAttempt::begin().await?;

    // @step And the Codex auth lock is held by another holder
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(fixture.directory.path().join(".fspec-auth.lock"))?;
    lock.lock()?;

    // @step And the attempt is committing its tokens and waits for the lock
    let commit = tokio::spawn(async move { attempt.commit(&tokens("stale")).await });
    tokio::task::yield_now().await;

    // @step When the login is cancelled
    let cancelled =
        deadlock_guard_blocking("cancel_login while the auth lock is held", cancel_login);

    // @step Then the cancellation returns while the lock is still held
    cancelled??;

    // @step And the commit fails as a changed login
    let committed = deadlock_guard("the cancelled commit", commit).await??;
    assert!(
        matches!(committed, Err(AuthError::Changed)),
        "a cancelled login committed: {committed:?}"
    );

    // @step And no tokens are stored once the lock is released
    lock.unlock()?;
    assert!(!stored_tokens()?, "a cancelled login stored tokens");
    Ok(())
}
