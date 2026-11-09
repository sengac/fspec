//! BUG-429 — --connect accepts a bearer token for URL daemons.
//!
//! Feature: spec/features/connect-url-bearer-token.feature

use super::resolve_connect;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[serial_test::serial]
#[test]
fn a_url_connection_carries_the_token_bearer() -> TestResult {
    // @step Given a --connect URL and a --token "s3cret"
    std::env::remove_var("FSPEC_DAEMON_TOKEN");
    // @step When the connection target is resolved
    let (url, bearer, _ca) =
        resolve_connect("wss://daemon.example:8443", Some("s3cret".to_string()))?;
    // @step Then the bearer is "s3cret"
    assert_eq!(url.as_str(), "wss://daemon.example:8443/");
    assert_eq!(bearer.as_deref(), Some("s3cret"));
    Ok(())
}

#[serial_test::serial]
#[test]
fn a_url_connection_without_token_uses_fspec_daemon_token() -> TestResult {
    // @step Given a --connect URL with no --token
    // @step And FSPEC_DAEMON_TOKEN is "from-env"
    std::env::set_var("FSPEC_DAEMON_TOKEN", "from-env");
    // @step When the connection target is resolved
    let resolved = resolve_connect("ws://127.0.0.1:9000", None);
    std::env::remove_var("FSPEC_DAEMON_TOKEN");
    // @step Then the bearer is "from-env"
    assert_eq!(resolved?.1.as_deref(), Some("from-env"));
    Ok(())
}
