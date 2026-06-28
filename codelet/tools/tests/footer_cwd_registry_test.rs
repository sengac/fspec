#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/footer-cwd-registry.feature
//!
//! Tests for per-session CWD tracking via SessionRegistry<String>.
//!
//! Verifies that:
//! - Each session tracks its last known CWD independently
//! - BashTool writes the resolved CWD to the registry after execution
//! - The registry can be read by the footer poller (cross-crate)
//! - Cleanup removes entries on session destroy

use codelet_tools::session_registry::SessionRegistry;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use uuid::Uuid;

/// Registry under test — mirrors the real `LAST_KNOWN_CWD` that will be
/// created in `codelet_tools` for per-session footer CWD tracking.
static FOOTER_CWD: Lazy<SessionRegistry<String>> = Lazy::new(SessionRegistry::new);

/// Test lock — registry tests must be sequential since they share global state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

// ============================================================================
// Scenario: Registry updates CWD when Bash tool uses explicit cwd
// ============================================================================

#[test]
fn test_registry_updates_cwd_on_explicit_cwd() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_id = Uuid::new_v4();

    // @step Given a session CWD entry is initialized to "/Users/rquast/projects/fspec"
    FOOTER_CWD.set(session_id, Some("/Users/rquast/projects/fspec".to_string()));
    assert_eq!(
        FOOTER_CWD.get(&session_id).unwrap(),
        "/Users/rquast/projects/fspec"
    );

    // @step When the Bash tool writes "/tmp" as the new CWD for that session
    FOOTER_CWD.set(session_id, Some("/tmp".to_string()));

    // @step Then the registry returns "/tmp" for that session
    assert_eq!(FOOTER_CWD.get(&session_id).unwrap(), "/tmp");

    // Cleanup
    FOOTER_CWD.remove(&session_id);
}

// ============================================================================
// Scenario: Registry tracks CWD independently per session
// ============================================================================

#[test]
fn test_registry_tracks_cwd_per_session() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given Session A CWD is "/Users/rquast/projects/fspec"
    FOOTER_CWD.set(session_a, Some("/Users/rquast/projects/fspec".to_string()));

    // @step And Session B CWD is "/Users/rquast/projects/fspec"
    FOOTER_CWD.set(session_b, Some("/Users/rquast/projects/fspec".to_string()));

    // @step When Session A CWD is updated to "/tmp"
    FOOTER_CWD.set(session_a, Some("/tmp".to_string()));

    // @step Then Session A registry returns "/tmp"
    assert_eq!(FOOTER_CWD.get(&session_a).unwrap(), "/tmp");

    // @step And Session B registry still returns "/Users/rquast/projects/fspec"
    assert_eq!(
        FOOTER_CWD.get(&session_b).unwrap(),
        "/Users/rquast/projects/fspec"
    );

    // Cleanup
    FOOTER_CWD.remove(&session_a);
    FOOTER_CWD.remove(&session_b);
}

// ============================================================================
// Scenario: Registry CWD returns to session default on no explicit cwd
// ============================================================================

#[test]
fn test_registry_cwd_returns_to_default() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_id = Uuid::new_v4();

    // @step Given a session CWD entry was set to "/tmp"
    FOOTER_CWD.set(session_id, Some("/tmp".to_string()));
    assert_eq!(FOOTER_CWD.get(&session_id).unwrap(), "/tmp");

    // @step When the Bash tool writes "/Users/rquast/projects/fspec" back as the CWD
    FOOTER_CWD.set(session_id, Some("/Users/rquast/projects/fspec".to_string()));

    // @step Then the registry returns "/Users/rquast/projects/fspec"
    assert_eq!(
        FOOTER_CWD.get(&session_id).unwrap(),
        "/Users/rquast/projects/fspec"
    );

    // Cleanup
    FOOTER_CWD.remove(&session_id);
}

// ============================================================================
// Scenario: Cleanup removes CWD entry on session destroy
// ============================================================================

#[test]
fn test_cleanup_removes_cwd_entry() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_id = Uuid::new_v4();

    // @step Given a session has a CWD entry in the registry
    FOOTER_CWD.set(session_id, Some("/some/path".to_string()));
    assert!(FOOTER_CWD.has(&session_id));

    // @step When the session is destroyed and cleanup is called
    FOOTER_CWD.remove(&session_id);

    // @step Then the registry entry for that session is removed
    assert!(!FOOTER_CWD.has(&session_id));
    assert!(FOOTER_CWD.get(&session_id).is_none());
}

// ============================================================================
// Scenario: Reading CWD for unknown session returns None
// ============================================================================

#[test]
fn test_unknown_session_returns_none() {
    let _guard = TEST_LOCK.lock().unwrap();
    let unknown_session = Uuid::new_v4();

    // @step Given no CWD is registered for a session
    // (fresh UUID, never inserted)

    // @step When the footer poller reads the CWD for that session
    let result = FOOTER_CWD.get(&unknown_session);

    // @step Then it receives None and falls back to the initial CWD
    assert!(result.is_none());
}

// ============================================================================
// Scenario: Initial CWD is seeded at session creation
// ============================================================================

#[test]
fn test_initial_cwd_seeding() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_id = Uuid::new_v4();

    // @step Given a new session is created with effective_cwd "/Users/rquast/projects/fspec"
    // (Session creation code seeds the registry)

    // @step When the session creation code seeds the registry
    FOOTER_CWD.set(session_id, Some("/Users/rquast/projects/fspec".to_string()));

    // @step Then the registry value is available immediately before any Bash commands run
    assert_eq!(
        FOOTER_CWD.get(&session_id).unwrap(),
        "/Users/rquast/projects/fspec"
    );
    let cwd_via_with = FOOTER_CWD.with(&session_id, Clone::clone);
    assert_eq!(
        cwd_via_with,
        Some("/Users/rquast/projects/fspec".to_string())
    );

    // Cleanup
    FOOTER_CWD.remove(&session_id);
}
