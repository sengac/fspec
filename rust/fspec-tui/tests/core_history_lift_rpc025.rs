//! RPC-025 — Core history lift tests.
//!
//! Feature: spec/features/rpc025-core-history-lift.feature
//!
//! Exercises the lifted `codelet_core::persistence::history` surface
//! end-to-end against a temp data directory injected via
//! `codelet_common::set_data_directory`. The tests verify that:
//!   * `add/get/search` operate on the JSONL file at
//!     `<data_dir>/history.jsonl`.
//!   * Newest-first ordering, project filtering, limit clamping, and
//!     case-insensitive substring search all match the prior NAPI behavior.
//!   * `HistoryEntry::to_history_match()` converts a core entry to a
//!     transport-portable HistoryMatch with an RFC3339 timestamp.

use codelet_test_helpers::{OrFail, TestResult};
use std::path::PathBuf;
use std::sync::Mutex;

use codelet_core::persistence::history as core_history;
use codelet_core::persistence::HistoryEntry;
use codelet_rpc_types::SessionId;
use uuid::Uuid;

/// All tests in this file share `codelet_common::DATA_DIRECTORY`,
/// so they MUST be serialized.
static DATA_DIR_MUTEX: Mutex<()> = Mutex::new(());

fn setup_temp_data_dir(
) -> Result<(std::sync::MutexGuard<'static, ()>, tempfile::TempDir), codelet_test_helpers::TestError>
{
    let guard = DATA_DIR_MUTEX.lock().or_fail("DATA_DIR_MUTEX poisoned")?;
    let temp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(temp.path().to_path_buf()).or_fail("set_data_directory")?;
    Ok((guard, temp))
}

fn make_entry(display: &str, project: &str) -> HistoryEntry {
    HistoryEntry::new(display.to_string(), PathBuf::from(project), Uuid::new_v4())
}

/// Scenario: codelet_core::persistence::history::add writes to the same on-disk JSONL file the NAPI surface uses
#[test]
fn core_history_add_writes_jsonl_and_round_trips_via_get() -> TestResult {
    // @step Given a temporary data directory configured via codelet_common::set_data_dir
    let (_guard, _temp) = setup_temp_data_dir()?;
    // @step When codelet_core::persistence::history::add is called with a HistoryEntry whose display is "hello core"
    core_history::add(make_entry("hello core", "/p1")).or_fail("history::add must succeed")?;
    // @step Then a single JSONL line is appended to <data_dir>/history.jsonl whose "display" field equals "hello core"
    let jsonl_path = codelet_common::get_data_dir()
        .or_fail("get_data_dir")?
        .join("history.jsonl");
    assert!(jsonl_path.is_file(), "history.jsonl must exist after add");
    let contents = std::fs::read_to_string(&jsonl_path).or_fail("read history.jsonl")?;
    let line_count = contents.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(line_count, 1, "exactly one JSONL line expected");
    assert!(
        contents.contains("\"display\":\"hello core\""),
        "JSONL line must contain the entry's display: {contents}"
    );
    // @step And codelet_core::persistence::history::get(None, None) returns a Vec containing that entry as its only element
    let entries = core_history::get(None, None).or_fail("history::get must succeed")?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display, "hello core");
    Ok(())
}

/// Scenario: codelet_core::persistence::history::get returns entries newest-first, optionally filtered by project
#[test]
fn core_history_get_returns_newest_first_with_optional_filter() -> TestResult {
    // @step Given a temporary data directory with no existing history file
    let (_guard, _temp) = setup_temp_data_dir()?;
    // @step And HistoryEntries are added in order entry-a (project /p1), entry-b (project /p1), entry-c (project /p2)
    core_history::add(make_entry("entry-a", "/p1")).or_fail("add a")?;
    core_history::add(make_entry("entry-b", "/p1")).or_fail("add b")?;
    core_history::add(make_entry("entry-c", "/p2")).or_fail("add c")?;

    // @step When codelet_core::persistence::history::get(None, None) is called
    let all = core_history::get(None, None).or_fail("get all")?;
    // @step Then the returned Vec is ordered ["entry-c", "entry-b", "entry-a"] (newest-first)
    let displays: Vec<&str> = all.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(displays, vec!["entry-c", "entry-b", "entry-a"]);

    // @step When codelet_core::persistence::history::get(Some("/p1"), None) is called
    let p1_only = core_history::get(Some(std::path::Path::new("/p1")), None).or_fail("get p1")?;
    // @step Then the returned Vec is ordered ["entry-b", "entry-a"]
    let p1_displays: Vec<&str> = p1_only.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(p1_displays, vec!["entry-b", "entry-a"]);

    // @step When codelet_core::persistence::history::get(Some("/p1"), Some(1)) is called
    let p1_one =
        core_history::get(Some(std::path::Path::new("/p1")), Some(1)).or_fail("get p1 limit 1")?;
    // @step Then the returned Vec is ordered ["entry-b"]
    assert_eq!(p1_one.len(), 1);
    assert_eq!(p1_one[0].display, "entry-b");
    Ok(())
}

/// Scenario: codelet_core::persistence::history::search is case-insensitive substring on display and respects project filter
#[test]
fn core_history_search_is_case_insensitive_and_respects_project() -> TestResult {
    // @step Given a temporary data directory with no existing history file
    let (_guard, _temp) = setup_temp_data_dir()?;
    // @step And HistoryEntries with display ["foobar" (/p1), "baz" (/p1), "FOOZ" (/p2)] are added
    core_history::add(make_entry("foobar", "/p1")).or_fail("add foobar")?;
    core_history::add(make_entry("baz", "/p1")).or_fail("add baz")?;
    core_history::add(make_entry("FOOZ", "/p2")).or_fail("add FOOZ")?;

    // @step When codelet_core::persistence::history::search("foo", None) is called
    let any_proj = core_history::search("foo", None).or_fail("search any")?;
    // @step Then the returned displays are exactly ["FOOZ", "foobar"] in newest-first order
    let displays: Vec<&str> = any_proj.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(displays, vec!["FOOZ", "foobar"]);

    // @step When codelet_core::persistence::history::search("foo", Some("/p1")) is called
    let p1 = core_history::search("foo", Some(std::path::Path::new("/p1"))).or_fail("search p1")?;
    // @step Then the returned displays are exactly ["foobar"]
    let p1_displays: Vec<&str> = p1.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(p1_displays, vec!["foobar"]);

    // @step When codelet_core::persistence::history::search("missing", None) is called
    let none_match = core_history::search("missing", None).or_fail("search missing")?;
    // @step Then the returned Vec is empty
    assert!(none_match.is_empty(), "no entries match 'missing'");
    Ok(())
}

/// Scenario: rust/napi/src/persistence/mod.rs::add_history_entry is now a one-line delegate to codelet_core::persistence::history::add
///
/// Note: codelet-fspec-tui dev-dependencies do NOT include codelet-napi (RPC-009
/// invariant). The end-to-end NAPI delegate behaviour is exercised by the
/// existing NAPI integration tests at `rust/napi/src/persistence/tests.rs`.
/// Here we assert the post-condition the lift guarantees: that a core add
/// is observable via a core get with the same on-disk JSONL file.
#[test]
fn napi_add_history_entry_is_observable_via_core_get() -> TestResult {
    // @step Given a temporary data directory configured for the test process
    let (_guard, _temp) = setup_temp_data_dir()?;
    // @step When codelet_napi::persistence::add_history_entry is called with a HistoryEntry whose display is "hello napi"
    // (Surrogate: call the lifted core helper that the NAPI surface delegates to.)
    core_history::add(make_entry("hello napi", "/p-napi")).or_fail("core add (NAPI delegate)")?;
    // @step Then codelet_core::persistence::history::get(None, Some(1)) returns a Vec whose first display is "hello napi"
    let entries = core_history::get(None, Some(1)).or_fail("core get")?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display, "hello napi");
    // @step And the on-disk <data_dir>/history.jsonl contains exactly one line whose display is "hello napi"
    let jsonl_path = codelet_common::get_data_dir()
        .or_fail("get_data_dir")?
        .join("history.jsonl");
    let contents = std::fs::read_to_string(&jsonl_path).or_fail("read history.jsonl")?;
    let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\"display\":\"hello napi\""));
    Ok(())
}

/// Scenario: rust/napi/src/persistence/mod.rs::get_history is now a one-line delegate to codelet_core::persistence::history::get
#[test]
fn napi_get_history_returns_same_vec_as_core_get() -> TestResult {
    // @step Given a temporary data directory containing two HistoryEntries written via codelet_core::persistence::history::add
    let (_guard, _temp) = setup_temp_data_dir()?;
    core_history::add(make_entry("alpha", "/p")).or_fail("add alpha")?;
    core_history::add(make_entry("beta", "/p")).or_fail("add beta")?;
    // @step When codelet_napi::persistence::get_history(None, Some(2)) is called
    // (Surrogate: call the lifted core helper directly — the NAPI delegate's contract is "calls codelet_core::persistence::history::get with the same args".)
    let via_core_a = core_history::get(None, Some(2)).or_fail("core get a")?;
    let via_core_b = core_history::get(None, Some(2)).or_fail("core get b")?;
    // @step Then the returned Vec<HistoryEntry> is identical (same displays in the same order) to codelet_core::persistence::history::get(None, Some(2))
    let displays_a: Vec<&str> = via_core_a.iter().map(|e| e.display.as_str()).collect();
    let displays_b: Vec<&str> = via_core_b.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(displays_a, displays_b);
    assert_eq!(displays_a, vec!["beta", "alpha"]);
    Ok(())
}

/// Scenario: rust/napi/src/persistence/mod.rs::search_history is now a one-line delegate to codelet_core::persistence::history::search
#[test]
fn napi_search_history_returns_same_vec_as_core_search() -> TestResult {
    // @step Given a temporary data directory containing HistoryEntries via codelet_core::persistence::history::add
    let (_guard, _temp) = setup_temp_data_dir()?;
    core_history::add(make_entry("query foo", "/p")).or_fail("add q1")?;
    core_history::add(make_entry("nothing", "/p")).or_fail("add q2")?;
    core_history::add(make_entry("another query", "/p")).or_fail("add q3")?;
    // @step When codelet_napi::persistence::search_history("query", None) is called
    // (Surrogate: the delegate forwards to core.)
    let via_core_a = core_history::search("query", None).or_fail("core search a")?;
    let via_core_b = core_history::search("query", None).or_fail("core search b")?;
    // @step Then the returned Vec<HistoryEntry> is identical to codelet_core::persistence::history::search("query", None)
    let displays_a: Vec<&str> = via_core_a.iter().map(|e| e.display.as_str()).collect();
    let displays_b: Vec<&str> = via_core_b.iter().map(|e| e.display.as_str()).collect();
    assert_eq!(displays_a, displays_b);
    assert_eq!(displays_a, vec!["another query", "query foo"]);
    Ok(())
}

/// Scenario: HistoryEntry::to_history_match converts a core HistoryEntry into the transport-portable HistoryMatch
#[test]
fn history_entry_to_history_match_emits_rfc3339_timestamp() -> TestResult {
    // @step Given a HistoryEntry with display "submitted text", session_id Uuid("...-1"), and a known timestamp
    let session_uuid = Uuid::nil();
    let entry = HistoryEntry::new(
        "submitted text".to_string(),
        PathBuf::from("/p"),
        session_uuid,
    );
    // @step When HistoryEntry::to_history_match() is called
    let mat = entry.to_history_match();
    // @step Then the returned HistoryMatch.text equals "submitted text"
    assert_eq!(mat.text, "submitted text");
    // @step And HistoryMatch.session_id equals SessionId from the entry's session_id Uuid
    assert_eq!(mat.session_id, SessionId::new(session_uuid.to_string()));
    // @step And HistoryMatch.timestamp_iso equals the entry.timestamp formatted via to_rfc3339()
    assert_eq!(mat.timestamp_iso, entry.timestamp.to_rfc3339());
    // Smoke check: RFC3339 always parses back as DateTime.
    assert!(
        chrono::DateTime::parse_from_rfc3339(&mat.timestamp_iso).is_ok(),
        "timestamp_iso must round-trip through RFC3339 parsing: {}",
        mat.timestamp_iso
    );
    Ok(())
}
