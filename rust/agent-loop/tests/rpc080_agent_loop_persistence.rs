//! Feature: spec/features/agent-loop-persistence.feature
//!
//! RPC-080: Agent loop persists user, assistant, tool_result, and token state per turn.
//!
//! These tests are organised in two groups:
//!
//!   - Behavioural tests (#[serial] + tempfile + set_data_directory) that
//!     drive the persist_* helpers directly against a hermetic on-disk
//!     manifest and read back the persisted MessageEnvelope shape via
//!     codelet_core::persistence::get_session_messages_full /
//!     load_session.
//!
//!   - Source-shape tests that scan the call-site files where each
//!     guarantee now lives to pin Rules [0]-[7] against future drift
//!     (durable input admission before dispatch, assistant-flush before
//!     tool_result, per-request token settlement, assistant flush on
//!     Done/Error/Interrupted, no codelet_napi::persist references).
//!
//! All behavioural tests serialise via `serial_test::serial` because
//! they reach for the process-global data directory + MESSAGE_STORE /
//! SESSION_STORE singletons in codelet_core::persistence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use codelet_core::persistence::{
    create_session, get_session_messages_full, load_session, reset_stores_for_tests,
    MessageEnvelope, StoredMessage,
};
use tempfile::TempDir;
use uuid::Uuid;

#[path = "rpc080_agent_loop_persistence/behaviour.rs"]
mod behaviour;
#[path = "rpc080_agent_loop_persistence/source_shape_arms.rs"]
mod source_shape_arms;
#[path = "rpc080_agent_loop_persistence/source_shape_turn.rs"]
mod source_shape_turn;

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

/// Create a hermetic session and return (session_id, _data_dir_guard).
fn fresh_session(name: &str) -> (Uuid, TempDir) {
    let guard = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc080");
    let session = create_session(name, &project).expect("create_session");
    (session.id, guard)
}

/// Read back the persisted stored messages for a session, then
/// reconstruct each MessageEnvelope by round-tripping the metadata
/// HashMap (which is exactly the envelope-as-JSON the persist_*
/// helpers wrote) through serde_json.
///
/// NOTE: `MessagePayload` is `#[serde(untagged)]` and `UserContent::Text`
/// / `AssistantContent::Text` are isomorphic on the wire, so
/// deserialising back into the typed enum can collapse an Assistant
/// envelope into the User variant. We therefore expose the raw
/// metadata JSON alongside the typed envelope so assistant-shape
/// scenarios can inspect `message.stop_reason` from the JSON directly.
fn read_envelopes(session_id: Uuid) -> Vec<MessageEnvelope> {
    let manifest = load_session(session_id).expect("load_session after persist");
    let stored: Vec<StoredMessage> =
        get_session_messages_full(&manifest).expect("get_session_messages_full");
    stored
        .into_iter()
        .map(|m| {
            let metadata_json = serde_json::to_value(&m.metadata)
                .expect("metadata HashMap must serialise back to JSON");
            serde_json::from_value::<MessageEnvelope>(metadata_json)
                .expect("metadata must round-trip back to a MessageEnvelope")
        })
        .collect()
}

/// Read the raw envelope JSON for each persisted message — preserves
/// the on-disk shape without being lossy on `#[serde(untagged)]`
/// variants.
fn read_envelope_jsons(session_id: Uuid) -> Vec<serde_json::Value> {
    let manifest = load_session(session_id).expect("load_session after persist");
    let stored: Vec<StoredMessage> =
        get_session_messages_full(&manifest).expect("get_session_messages_full");
    stored
        .into_iter()
        .map(|m| {
            serde_json::to_value(&m.metadata).expect("metadata HashMap must serialise back to JSON")
        })
        .collect()
}

// ============================================================================
// Source-shape helpers
// ============================================================================

/// Raw text of one file under rust/agent-loop/src/ (textual order matters
/// for the ordering checks, so child modules are not spliced in).
fn read_agent_loop_src(relative: &str) -> String {
    read_raw(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(relative),
    )
}

/// Raw text of one file in a sibling workspace crate, relative to rust/.
fn read_workspace_src(relative: &str) -> String {
    read_raw(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(relative),
    )
}

fn read_raw(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {e}", path.display()))
}

/// The text from the first `start` up to the next `end` after it.
fn section<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
    let from = src
        .find(start)
        .unwrap_or_else(|| panic!("`{start}` must exist"));
    let to = src[from..]
        .find(end)
        .map(|i| i + from)
        .unwrap_or_else(|| panic!("`{start}` must be followed by `{end}`"));
    &src[from..to]
}

/// Byte offset of `needle` in `haystack`, failing with `what`.
fn position(haystack: &str, needle: &str, what: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("{what} must contain `{needle}`"))
}

/// Recursively collect all .rs files under `dir`.
fn walk_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("must read_dir {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rs_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}
