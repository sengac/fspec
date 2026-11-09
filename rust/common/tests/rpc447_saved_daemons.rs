//! RPC-447: saved remote daemons.
//!
//! Feature: spec/features/saved-remote-daemons.feature

use codelet_common::saved_daemons::{self, SavedDaemon};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn daemon(name: &str, url: &str, token: Option<&str>) -> SavedDaemon {
    SavedDaemon {
        name: name.to_string(),
        url: url.to_string(),
        token: token.map(str::to_string),
        ca: None,
    }
}

#[test]
fn saving_a_remote_daemon_stores_it_owner_only() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When the daemon "work" is saved with URL "wss://fspec.example.com" and token "s3cret"
    saved_daemons::save(
        &path,
        daemon("work", "wss://fspec.example.com", Some("s3cret")),
    )?;

    // @step Then daemons.json lists "work" with that URL and token
    let saved = saved_daemons::list(&path)?;
    assert_eq!(
        saved,
        vec![daemon("work", "wss://fspec.example.com", Some("s3cret"))]
    );

    // @step And daemons.json is readable and writable only by its owner
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "daemons.json mode was {mode:o}");
    }
    Ok(())
}

#[test]
fn a_plain_ws_url_to_a_non_loopback_host_is_rejected() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When the daemon "lan" is saved with URL "ws://10.0.0.5:9000"
    let result = saved_daemons::save(&path, daemon("lan", "ws://10.0.0.5:9000", Some("t")));

    // @step Then saving fails because remote daemons must use wss://
    let error = result.err().ok_or("saving must fail")?.to_string();
    assert!(error.contains("wss://"), "{error}");

    // @step And nothing is written
    assert!(!path.exists());
    Ok(())
}

#[test]
fn a_wss_url_without_a_token_is_rejected() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When the daemon "work" is saved with URL "wss://fspec.example.com" and no token
    let result = saved_daemons::save(&path, daemon("work", "wss://fspec.example.com", None));

    // @step Then saving fails because remote daemons need a token
    let error = result.err().ok_or("saving must fail")?.to_string();
    assert!(error.contains("token"), "{error}");
    Ok(())
}

#[test]
fn a_loopback_ws_url_needs_no_token() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When the daemon "local" is saved with URL "ws://127.0.0.1:9000" and no token
    saved_daemons::save(&path, daemon("local", "ws://127.0.0.1:9000", None))?;

    // @step Then daemons.json lists "local"
    let names: Vec<String> = saved_daemons::list(&path)?
        .into_iter()
        .map(|d| d.name)
        .collect();
    assert_eq!(names, ["local"]);
    Ok(())
}

#[test]
fn removing_a_saved_daemon() -> TestResult {
    // @step Given saved daemons "work" and "local"
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");
    saved_daemons::save(
        &path,
        daemon("work", "wss://fspec.example.com", Some("s3cret")),
    )?;
    saved_daemons::save(&path, daemon("local", "ws://127.0.0.1:9000", None))?;

    // @step When "work" is removed
    let removed = saved_daemons::remove(&path, "work")?;

    // @step Then only "local" remains
    assert!(removed);
    let names: Vec<String> = saved_daemons::list(&path)?
        .into_iter()
        .map(|d| d.name)
        .collect();
    assert_eq!(names, ["local"]);
    Ok(())
}
