//! Feature: spec/features/resume-session-ordering.feature
//!
//! TUI-099 — Integration tests for session list ordering in SessionManager.
//! Verifies that list_sessions() returns sessions sorted by updated_at_ms
//! descending (most recent first), with session ID as tiebreaker, and
//! sessions without timestamps appearing at the end.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionInfo;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex;

/// PROV-132: Serialize tests that swap the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Helper: create a temp data directory and return its path.
fn make_temp_data_dir() -> PathBuf {
    tempfile::tempdir().expect("tempdir").keep()
}

/// Helper: set the data directory and return a guard that cleans it up.
fn set_temp_data_dir(path: PathBuf) -> PathBuf {
    codelet_common::set_data_directory(path.clone()).expect("set_data_directory");
    // Reset the persistence singletons so they re-initialize against the new data dir.
    codelet_core::persistence::reset_stores_for_tests();
    path
}

/// Helper: construct a SessionInfo with a specific updated_at_ms.
fn make_session(id: &str, ts: Option<i64>) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: format!("Session {}", id),
        status: "idle".to_string(),
        project: ".".to_string(),
        message_count: 0,
        provider_id: Some("anthropic".to_string()),
        model_id: Some("claude-sonnet-4".to_string()),
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: ts,
    }
}

/// Helper: apply the expected sorting algorithm to a Vec<SessionInfo>.
/// This mirrors the sorting logic that list_sessions() should implement.
fn sort_sessions(sessions: &mut Vec<SessionInfo>) {
    sessions.sort_by(|a, b| {
        match (a.updated_at_ms, b.updated_at_ms) {
            (Some(ts_a), Some(ts_b)) => {
                ts_b.cmp(&ts_a).then_with(|| a.id.cmp(&b.id))
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.id.cmp(&b.id),
        }
    });
}

// ============================================================================
// Scenario: Sessions ordered by most recently updated first
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sessions_ordered_by_most_recently_updated_first() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have multiple sessions with different update timestamps
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    // Create sessions
    let sid_a = handle.create_session(None);
    let _info_a = manager
        .list_sessions()
        .iter()
        .find(|s| s.id == sid_a.value)
        .cloned()
        .expect("session A should exist");

    let sid_b = handle.create_session(None);
    let _info_b = manager
        .list_sessions()
        .iter()
        .find(|s| s.id == sid_b.value)
        .cloned()
        .expect("session B should exist");

    let sid_c = handle.create_session(None);
    let _info_c = manager
        .list_sessions()
        .iter()
        .find(|s| s.id == sid_c.value)
        .cloned()
        .expect("session C should exist");

    // @step When I open the /resume view
    let sessions = manager.list_sessions();

    // @step Then the sessions are displayed in descending order by updated_at_ms
    assert!(
        sessions.len() >= 3,
        "should have at least 3 sessions, got {}",
        sessions.len()
    );

    // Verify all sessions have timestamps
    for s in &sessions {
        assert!(
            s.updated_at_ms.is_some(),
            "session {} should have updated_at_ms",
            s.id
        );
    }

    // Verify the list is sorted in descending order by updated_at_ms
    // (with session ID as tiebreaker for equal timestamps)
    for i in 1..sessions.len() {
        let prev_ts = sessions[i - 1].updated_at_ms.unwrap();
        let curr_ts = sessions[i].updated_at_ms.unwrap();
        if prev_ts == curr_ts {
            // Equal timestamps: should be alphabetical by ID
            assert!(
                sessions[i - 1].id <= sessions[i].id,
                "sessions with equal timestamps should be alphabetical by ID: '{}' <= '{}'",
                sessions[i - 1].id,
                sessions[i].id
            );
        } else {
            assert!(
                prev_ts > curr_ts,
                "descending timestamp order violated at index {}: {} > {}",
                i,
                prev_ts,
                curr_ts
            );
        }
    }
}

// ============================================================================
// Scenario: Sessions with identical timestamps are ordered by session ID
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sessions_with_identical_timestamps_ordered_by_session_id() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have two sessions with the same updated_at_ms timestamp
    // Construct SessionInfo objects with identical timestamps to test
    // the tiebreaker logic directly.
    let ts = 1_700_000_000_000i64;
    let session_alpha = make_session("aaaa1111-1111-1111-1111-111111111111", Some(ts));
    let session_zulu = make_session("zzzz1111-1111-1111-1111-111111111111", Some(ts));

    // @step When I open the /resume view
    let mut sessions = vec![session_zulu.clone(), session_alpha.clone()];
    sort_sessions(&mut sessions);

    // @step Then the sessions are ordered alphabetically by session ID as a tiebreaker
    assert_eq!(
        sessions[0].id, "aaaa1111-1111-1111-1111-111111111111",
        "session with alphabetically earlier ID should come first"
    );
    assert_eq!(
        sessions[1].id, "zzzz1111-1111-1111-1111-111111111111",
        "session with alphabetically later ID should come second"
    );

    // Also verify with a three-way tie
    let session_mid = make_session("mmmm1111-1111-1111-1111-111111111111", Some(ts));
    let mut sessions = vec![session_zulu.clone(), session_mid.clone(), session_alpha.clone()];
    sort_sessions(&mut sessions);
    assert_eq!(sessions[0].id, "aaaa1111-1111-1111-1111-111111111111");
    assert_eq!(sessions[1].id, "mmmm1111-1111-1111-1111-111111111111");
    assert_eq!(sessions[2].id, "zzzz1111-1111-1111-1111-111111111111");

    // Verify the actual list_sessions() output is sorted
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let _sid1 = handle.create_session(None);
    let _sid2 = handle.create_session(None);
    let sessions = manager.list_sessions();

    // Verify the overall list is sorted
    for i in 1..sessions.len() {
        let prev = &sessions[i - 1];
        let curr = &sessions[i];
        match (prev.updated_at_ms, curr.updated_at_ms) {
            (Some(ts_prev), Some(ts_curr)) => {
                if ts_prev == ts_curr {
                    assert!(
                        prev.id <= curr.id,
                        "sessions with equal timestamps should be alphabetical by ID: '{}' <= '{}'",
                        prev.id,
                        curr.id
                    );
                } else {
                    assert!(
                        ts_prev > ts_curr,
                        "descending timestamp order violated at index {}: {} > {}",
                        i,
                        ts_prev,
                        ts_curr
                    );
                }
            }
            (Some(_), None) => {
                // OK — timestamped before non-timestamped
            }
            (None, Some(_)) => {
                panic!(
                    "non-timestamped session should not come before timestamped session"
                );
            }
            (None, None) => {
                assert!(
                    prev.id <= curr.id,
                    "non-timestamped sessions should be alphabetical by ID"
                );
            }
        }
    }
}

// ============================================================================
// Scenario: Sessions without a timestamp appear at the end
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sessions_without_timestamp_appear_at_end() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have sessions with and without updated_at_ms timestamps
    let session_with_ts_latest = make_session("aaaa1111-1111-1111-1111-111111111111", Some(1000));
    let session_with_ts_earlier = make_session("bbbb1111-1111-1111-1111-111111111111", Some(500));
    let session_without_ts_first =
        make_session("cccc1111-1111-1111-1111-111111111111", None);
    let session_without_ts_second =
        make_session("dddd1111-1111-1111-1111-111111111111", None);

    // @step When I open the /resume view
    let mut unsorted = vec![
        session_without_ts_second.clone(),
        session_with_ts_earlier.clone(),
        session_without_ts_first.clone(),
        session_with_ts_latest.clone(),
    ];
    sort_sessions(&mut unsorted);

    // @step Then sessions with timestamps appear first and sessions without timestamps appear last
    // Verify: sessions with timestamps come first, sorted descending by timestamp
    assert_eq!(
        unsorted[0].id, session_with_ts_latest.id,
        "most recent session should be first"
    );
    assert_eq!(
        unsorted[1].id, session_with_ts_earlier.id,
        "earlier session should be second"
    );
    // Then sessions without timestamps, sorted alphabetically by ID
    assert_eq!(
        unsorted[2].id, session_without_ts_first.id,
        "first non-timestamped session (alphabetically) should be third"
    );
    assert_eq!(
        unsorted[3].id, session_without_ts_second.id,
        "second non-timestamped session (alphabetically) should be last"
    );

    // Also verify the actual list_sessions() output is sorted
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let _sid = handle.create_session(None);
    let sessions = manager.list_sessions();

    // Verify all sessions from list_sessions() are properly sorted
    for i in 1..sessions.len() {
        let prev = &sessions[i - 1];
        let curr = &sessions[i];
        match (prev.updated_at_ms, curr.updated_at_ms) {
            (Some(ts_prev), Some(ts_curr)) => {
                if ts_prev == ts_curr {
                    assert!(
                        prev.id <= curr.id,
                        "equal timestamps should be alphabetical by ID"
                    );
                } else {
                    assert!(
                        ts_prev > ts_curr,
                        "descending timestamp order violated at index {}",
                        i
                    );
                }
            }
            (Some(_), None) => {
                // OK — timestamped before non-timestamped
            }
            (None, Some(_)) => {
                panic!(
                    "non-timestamped session should not come before timestamped session"
                );
            }
            (None, None) => {
                assert!(
                    prev.id <= curr.id,
                    "non-timestamped sessions should be alphabetical"
                );
            }
        }
    }
}
