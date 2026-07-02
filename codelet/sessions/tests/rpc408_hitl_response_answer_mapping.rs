//! Feature: spec/features/hitl-response-answer-mapping.feature
//!
//! RPC-408 — Behavioural tests for `SessionManagerHandle::send_hitl_response`.
//! Each `#[test]` maps 1:1 to a Gherkin scenario in the feature file.
//!
//! The tests construct a fresh `SessionManager`, create a session via
//! the trait's `create_session` bridge (Noop hooks, no agent loop),
//! store a pending internal HITL request on the `BackgroundSession`,
//! block a std thread on `wait_for_hitl_response()`, then drive the
//! handle's `send_hitl_response` with wire `HitlResponse{id, value}`
//! payloads and assert the internal response the blocked thread
//! receives is `Answered` with the mapped payload — never `Cancelled`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{HitlResponse as WireHitlResponse, SessionId};
use codelet_sessions::SessionManager;
use codelet_tools::request_user_input::{
    HitlOption, HitlQuestion, HitlRequest as InternalHitlRequest,
    HitlResponse as InternalHitlResponse,
};

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// The Noop hooks ensure no agent loop is spawned for the session.
async fn fresh_session(manager: &SessionManager) -> SessionId {
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    handle.create_session(None)
}

/// A pending internal HITL request with one question `confirm_choice`
/// offering options `Yes` / `No` (text input allowed on the wire shape).
fn pending_request() -> InternalHitlRequest {
    InternalHitlRequest {
        questions: vec![HitlQuestion {
            id: "confirm_choice".to_string(),
            header: "Confirm".to_string(),
            question: "Proceed with the plan?".to_string(),
            options: Some(vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "Go ahead".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "Stop here".to_string(),
                },
            ]),
        }],
    }
}

/// Spawn a std thread blocked on `wait_for_hitl_response()` for the
/// given session and return its join handle. The thread returns the
/// internal response it receives.
fn spawn_blocked_waiter(
    manager: &Arc<SessionManager>,
    sid: &SessionId,
) -> std::thread::JoinHandle<InternalHitlResponse> {
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist for waiter");
    std::thread::spawn(move || session.wait_for_hitl_response())
}

/// Join the waiter with a timeout so a broken send path fails the test
/// instead of hanging it forever.
fn join_waiter(waiter: std::thread::JoinHandle<InternalHitlResponse>) -> InternalHitlResponse {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !waiter.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "wait_for_hitl_response never unblocked — send_hitl_response did not deliver"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    waiter.join().expect("waiter thread must not panic")
}

// ============================================================================
// Scenario: Selecting an option label delivers Answered with that label
// selected
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn option_label_maps_to_answered_selected() {
    // @step Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(pending_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with id "confirm_choice" and value "Yes"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle
        .send_hitl_response(
            &sid,
            WireHitlResponse {
                id: "confirm_choice".to_string(),
                value: "Yes".to_string(),
            },
        )
        .expect("send_hitl_response must succeed");

    // @step Then the blocked thread receives an Answered response
    let response = join_waiter(waiter);
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };

    // @step And the answer for question "confirm_choice" has selected equal to ["Yes"] and other equal to None
    let answer = answers
        .get("confirm_choice")
        .expect("answer must be keyed by the pending question id");
    assert_eq!(answer.selected, vec!["Yes".to_string()]);
    assert_eq!(answer.other, None);
}

// ============================================================================
// Scenario: Typing free text delivers Answered with the text as other
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn free_text_maps_to_answered_other() {
    // @step Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(pending_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with id "confirm_choice" and value "maybe later"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle
        .send_hitl_response(
            &sid,
            WireHitlResponse {
                id: "confirm_choice".to_string(),
                value: "maybe later".to_string(),
            },
        )
        .expect("send_hitl_response must succeed");

    // @step Then the blocked thread receives an Answered response
    let response = join_waiter(waiter);
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };

    // @step And the answer for question "confirm_choice" has selected equal to [] and other equal to Some("maybe later")
    let answer = answers
        .get("confirm_choice")
        .expect("answer must be keyed by the pending question id");
    assert!(answer.selected.is_empty());
    assert_eq!(answer.other, Some("maybe later".to_string()));
}

// ============================================================================
// Scenario: Mismatched response id still answers keyed by the pending
// question id
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mismatched_id_answers_keyed_by_pending_question_id() {
    // @step Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(pending_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with id "stale_id" and value "No"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle
        .send_hitl_response(
            &sid,
            WireHitlResponse {
                id: "stale_id".to_string(),
                value: "No".to_string(),
            },
        )
        .expect("send_hitl_response must succeed");

    // @step Then the blocked thread receives an Answered response
    let response = join_waiter(waiter);
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };

    // @step And the answer is keyed by "confirm_choice" and has selected equal to ["No"] and other equal to None
    assert!(
        !answers.contains_key("stale_id"),
        "answer must not be keyed by the stale response id"
    );
    let answer = answers
        .get("confirm_choice")
        .expect("answer must be keyed by the pending question id");
    assert_eq!(answer.selected, vec!["No".to_string()]);
    assert_eq!(answer.other, None);
}

// ============================================================================
// Scenario: No pending HITL request falls back to the response id and free
// text
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_pending_request_falls_back_to_response_id_free_text() {
    // @step Given a BackgroundSession with no pending HITL request stored
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(None);

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with id "orphan_question" and value "some answer"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle
        .send_hitl_response(
            &sid,
            WireHitlResponse {
                id: "orphan_question".to_string(),
                value: "some answer".to_string(),
            },
        )
        .expect("send_hitl_response must succeed");

    // @step Then the blocked thread receives an Answered response
    let response = join_waiter(waiter);
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };

    // @step And the answer for question "orphan_question" has selected equal to [] and other equal to Some("some answer")
    let answer = answers
        .get("orphan_question")
        .expect("answer must fall back to the response id when no request is pending");
    assert!(answer.selected.is_empty());
    assert_eq!(answer.other, Some("some answer".to_string()));
}

// ============================================================================
// Scenario: Option label versus free text discrimination is the wire-path
// parity contract
// ============================================================================

/// Wire-path parity contract with the napi path.
///
/// The napi `session_send_hitl_response`
/// (codelet/napi/src/session_bindings.rs:1720-1750) maps a selected
/// option label to `HitlAnswer{selected:[label], other:None}` and free
/// text to `HitlAnswer{selected:[], other:Some(text)}`. This single
/// table-driven test locks the identical discrimination on the wire
/// path so both frontends produce identical Answered payloads for
/// identical user actions (Rule 6).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn option_label_vs_free_text_discrimination_table() {
    // @step Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");

    // Discrimination table: (wire value, expected selected, expected other)
    let table: &[(&str, &[&str], Option<&str>)] = &[
        ("Yes", &["Yes"], None),
        ("No", &["No"], None),
        ("anything else", &[], Some("anything else")),
    ];

    // @step When the handle receives send_hitl_response with value "Yes", then "No", then "anything else"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    for (value, expected_selected, expected_other) in table {
        session.set_hitl_request(Some(pending_request()));
        let waiter = spawn_blocked_waiter(&manager, &sid);
        handle
            .send_hitl_response(
                &sid,
                WireHitlResponse {
                    id: "confirm_choice".to_string(),
                    value: (*value).to_string(),
                },
            )
            .expect("send_hitl_response must succeed");

        let response = join_waiter(waiter);
        let InternalHitlResponse::Answered { answers } = response else {
            panic!("value {value:?}: expected Answered, got Cancelled: {response:?}");
        };
        let answer = answers
            .get("confirm_choice")
            .expect("answer must be keyed by the pending question id");

        // @step Then value "Yes" maps to an Answered answer with selected equal to ["Yes"] and other equal to None
        // @step And value "No" maps to an Answered answer with selected equal to ["No"] and other equal to None
        // @step And value "anything else" maps to an Answered answer with selected equal to [] and other equal to Some("anything else")
        let expected_selected: Vec<String> = expected_selected
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        assert_eq!(
            answer.selected, expected_selected,
            "value {value:?}: selected must match the parity contract"
        );
        assert_eq!(
            answer.other,
            expected_other.map(std::string::ToString::to_string),
            "value {value:?}: other must match the parity contract"
        );
    }
}

// ============================================================================
// Scenario: send_hitl_response never delivers Cancelled
// ============================================================================

#[test]
fn send_hitl_response_source_no_longer_hardcodes_cancelled() {
    // @step Given the source of handle_impl.rs::send_hitl_response
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/handle_impl.rs");

    // @step When the source of send_hitl_response is inspected
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let fn_start = src
        .find("fn send_hitl_response(")
        .expect("handle_impl.rs must define send_hitl_response");
    // Bound the slice at the next trait method after send_hitl_response.
    let fn_end = src[fn_start..]
        .find("fn get_pause_state(")
        .map(|off| fn_start + off)
        .unwrap_or(src.len());
    let body = &src[fn_start..fn_end];

    // @step Then it no longer contains the hard-coded Cancelled response
    assert!(
        !body.contains("Cancelled {"),
        "handle_impl.rs::send_hitl_response must not construct any \
         `Cancelled {{ .. }}` response — the wire path has no cancel \
         affordance and must always map to Answered (RPC-408)"
    );

    // @step And every response delivered from this path is an Answered variant
    assert!(
        body.contains("Answered"),
        "handle_impl.rs::send_hitl_response must construct \
         HitlResponse::Answered from the wire payload (RPC-408)"
    );
}
