//! BUG-595 — the daemon switcher lists a local TLS daemon with the scheme
//! and host its discovery entry names.
//!
//! Feature: spec/features/local-daemon-discovery-names-the-scheme-and-host.feature

use crate::common::{DaemonEntry, DaemonScheme};

#[test]
fn the_daemon_switcher_lists_a_local_tls_daemon_with_its_scheme_and_host() {
    // @step Given a live local daemon entry with "scheme": "wss" and "host": "localhost"
    let entry = DaemonEntry {
        port: 3777,
        pid: std::process::id(),
        workspace: Some("/work/alpha".to_string()),
        started_at: None,
        version: None,
        scheme: DaemonScheme::Wss,
        host: "localhost".to_string(),
    };

    // @step When the switcher builds its local daemon choice
    let choice = super::local_choice(&entry);

    // @step Then the choice's URL is wss://localhost:<port>
    assert_eq!(choice.url, "wss://localhost:3777");
    assert_eq!(choice.name, "alpha:3777");
    assert!(!choice.saved);
}
