//! Feature: spec/features/hitl-wire-protocol-parity.feature
//!
//! RPC-410 — Behavioural tests for the TS-parity HITL wire protocol:
//! `SessionManagerHandle::get_hitl_request` surfaces the FULL questions
//! array (no first-question slicing) and
//! `SessionManagerHandle::send_hitl_response` is a direct pass-through
//! (`cancelled` → internal `Cancelled`, answers vec → `Answered`
//! HashMap keyed by answer id — NO option-label inference).
//!
//! The tests construct a fresh `SessionManager`, create a session via
//! the trait's `create_session` bridge (Noop hooks, no agent loop),
//! store a pending internal HITL request on the `BackgroundSession`,
//! block a std thread on `wait_for_hitl_response()`, then drive the
//! handle with new wire `HitlResponse{cancelled, answers}` payloads.
//!
//! HANG-SAFETY (see codelet/sessions/tests/paused_chunk_delivery_rpc409.rs
//! module docs): every scenario that parks a waiter UNCONDITIONALLY
//! sends the unblocking response BEFORE any assertion, and every join
//! is bounded by a hard deadline (`join_waiter`) so a broken send path
//! FAILS the test instead of hanging it. Never assert while a waiter
//! is still parked.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{
    HitlAnswer as WireHitlAnswer, HitlOption as WireHitlOption, HitlQuestion as WireHitlQuestion,
    HitlRequest as WireHitlRequest, HitlResponse as WireHitlResponse, SessionId,
};
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

/// One internal question with two options.
fn option_question(id: &str, header: &str, question: &str) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: question.to_string(),
        options: Some(vec![
            HitlOption {
                label: "Option A".to_string(),
                description: "First choice".to_string(),
            },
            HitlOption {
                label: "Option B".to_string(),
                description: "Second choice".to_string(),
            },
        ]),
    }
}

/// One internal freeform question (options: None).
fn freeform_question(id: &str, header: &str, question: &str) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: question.to_string(),
        options: None,
    }
}

/// A pending internal 3-question request (approach / priority / notes;
/// notes is freeform).
fn three_question_request() -> InternalHitlRequest {
    InternalHitlRequest {
        questions: vec![
            option_question("approach", "Approach", "Which approach do you prefer?"),
            option_question("priority", "Priority", "What is the priority?"),
            freeform_question("notes", "Notes", "Any additional notes?"),
        ],
    }
}

/// A pending internal request with a single Yes/No question.
fn yes_no_request() -> InternalHitlRequest {
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

/// Join the waiter with a hard deadline so a broken send path fails
/// the test instead of hanging it forever. Only call this AFTER the
/// unblocking `send_hitl_response` was issued.
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
// Scenario: A three-question internal request surfaces all questions over
// the wire in order
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_question_request_surfaces_all_questions_in_order() {
    // @step Given a BackgroundSession with a pending internal HITL request containing questions "approach", "priority" and "notes"
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(three_question_request()));

    // @step When the handle's get_hitl_request is called for that session
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let wire = handle
        .get_hitl_request(&sid)
        .expect("a pending request must surface on the wire");

    // @step Then the wire HitlRequest contains exactly 3 questions with ids "approach", "priority" and "notes" in order
    let ids: Vec<&str> = wire.questions.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(ids, vec!["approach", "priority", "notes"]);

    // @step And each wire question preserves the internal header, question text and option labels
    assert_eq!(wire.questions[0].header, "Approach");
    assert_eq!(wire.questions[0].question, "Which approach do you prefer?");
    let labels: Vec<&str> = wire.questions[0]
        .options
        .iter()
        .map(|o| o.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Option A", "Option B"]);
    assert_eq!(wire.questions[1].header, "Priority");
    assert_eq!(wire.questions[1].question, "What is the priority?");
    assert_eq!(wire.questions[2].header, "Notes");
    assert_eq!(wire.questions[2].question, "Any additional notes?");
}

// ============================================================================
// Scenario: A question without options surfaces an empty options array on
// the wire
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn question_without_options_surfaces_empty_options_array() {
    // @step Given a BackgroundSession with a pending internal HITL request whose question "notes" has options None
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(InternalHitlRequest {
        questions: vec![freeform_question("notes", "Notes", "Any additional notes?")],
    }));

    // @step When the handle's get_hitl_request is called for that session
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let wire = handle
        .get_hitl_request(&sid)
        .expect("a pending request must surface on the wire");

    // @step Then the wire question "notes" has an empty options array
    let notes = wire
        .questions
        .iter()
        .find(|q| q.id == "notes")
        .expect("question 'notes' must surface on the wire");
    assert!(
        notes.options.is_empty(),
        "options: None must surface as an empty vec, got {:?}",
        notes.options
    );
}

// ============================================================================
// Scenario: A cancelled wire response reaches the blocked tool as Cancelled
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_wire_response_reaches_blocked_tool_as_cancelled() {
    // @step Given a BackgroundSession with a pending internal HITL request
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(yes_no_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with cancelled true and no answers
    // HANG-SAFETY: capture the send outcome WITHOUT panicking so the
    // waiter is never stranded behind an assert; join_waiter below is
    // deadline-bounded either way.
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let send_result = handle.send_hitl_response(
        &sid,
        WireHitlResponse {
            cancelled: true,
            answers: vec![],
        },
    );

    // @step Then the blocked thread receives a Cancelled response with cancelled true
    let response = join_waiter(waiter);
    assert!(send_result.is_ok(), "send_hitl_response must succeed");
    assert_eq!(
        response,
        InternalHitlResponse::Cancelled { cancelled: true },
        "cancelled:true on the wire must reach the tool as Cancelled{{cancelled:true}}"
    );
}

// ============================================================================
// Scenario: A structured multi-answer wire response maps pass-through to
// Answered keyed by answer id
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_answer_wire_response_maps_pass_through_keyed_by_answer_id() {
    // @step Given a BackgroundSession with a pending internal HITL request containing questions "approach" and "notes"
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(InternalHitlRequest {
        questions: vec![
            option_question("approach", "Approach", "Which approach do you prefer?"),
            freeform_question("notes", "Notes", "Any additional notes?"),
        ],
    }));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with answers for "approach" selecting "Option A" and for "notes" with other "free text"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let send_result = handle.send_hitl_response(
        &sid,
        WireHitlResponse {
            cancelled: false,
            answers: vec![
                WireHitlAnswer {
                    id: "approach".to_string(),
                    selected: vec!["Option A".to_string()],
                    other: None,
                },
                WireHitlAnswer {
                    id: "notes".to_string(),
                    selected: vec![],
                    other: Some("free text".to_string()),
                },
            ],
        },
    );

    // @step Then the blocked thread receives an Answered response with a 2-entry map keyed by "approach" and "notes"
    let response = join_waiter(waiter);
    assert!(send_result.is_ok(), "send_hitl_response must succeed");
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };
    assert_eq!(answers.len(), 2, "both answers must be delivered");

    // @step And the answer for "approach" has selected equal to ["Option A"] and other equal to None
    let approach = answers
        .get("approach")
        .expect("answer must be keyed by the wire answer id 'approach'");
    assert_eq!(approach.selected, vec!["Option A".to_string()]);
    assert_eq!(approach.other, None);

    // @step And the answer for "notes" has selected equal to [] and other equal to Some("free text")
    let notes = answers
        .get("notes")
        .expect("answer must be keyed by the wire answer id 'notes'");
    assert!(notes.selected.is_empty());
    assert_eq!(notes.other, Some("free text".to_string()));
}

// ============================================================================
// Scenario: Freeform text identical to an option label stays in other
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn freeform_text_identical_to_option_label_stays_in_other() {
    // @step Given a BackgroundSession with a pending internal HITL request whose question "confirm_choice" has options "Yes" and "No"
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(yes_no_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with an answer for "confirm_choice" with empty selected and other "Yes"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let send_result = handle.send_hitl_response(
        &sid,
        WireHitlResponse {
            cancelled: false,
            answers: vec![WireHitlAnswer {
                id: "confirm_choice".to_string(),
                selected: vec![],
                other: Some("Yes".to_string()),
            }],
        },
    );

    // @step Then the blocked thread receives an Answered response
    let response = join_waiter(waiter);
    assert!(send_result.is_ok(), "send_hitl_response must succeed");
    let InternalHitlResponse::Answered { answers } = response else {
        panic!("expected Answered, got Cancelled: {response:?}");
    };

    // @step And the answer for "confirm_choice" has selected equal to [] and other equal to Some("Yes")
    let answer = answers
        .get("confirm_choice")
        .expect("answer must be keyed by the wire answer id");
    assert!(
        answer.selected.is_empty(),
        "free text equal to an option label must NOT be reclassified as a \
         selection (RPC-408 heuristic regression), got selected={:?}",
        answer.selected
    );
    assert_eq!(answer.other, Some("Yes".to_string()));
}

// ============================================================================
// Scenario: The send path contains no option-label comparison
// ============================================================================

#[test]
fn send_path_contains_no_option_label_comparison() {
    // @step Given the source of handle_impl.rs and the hitl mapping module
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let handle_src = std::fs::read_to_string(manifest.join("src/handle_impl.rs"))
        .expect("handle_impl.rs must exist");
    let mapping_path = manifest.join("src/hitl_mapping.rs");
    let mapping_src = std::fs::read_to_string(&mapping_path).unwrap_or_else(|e| {
        panic!(
            "sessions/src/hitl_mapping.rs must exist (dossier §3.2/§3.3 \
             extraction): {e}"
        )
    });

    // @step When the send_hitl_response path is inspected
    let fn_start = handle_src
        .find("fn send_hitl_response(")
        .expect("handle_impl.rs must define send_hitl_response");
    let fn_end = handle_src[fn_start..]
        .find("fn get_pause_state(")
        .map(|off| fn_start + off)
        .unwrap_or(handle_src.len());
    let body = &handle_src[fn_start..fn_end];
    let send_path = format!("{body}\n{mapping_src}");

    // @step Then it contains no comparison of answer values against option labels
    for needle in ["is_option_label", ".label ==", "label == "] {
        assert!(
            !send_path.contains(needle),
            "the send path must not compare answer values against option \
             labels (found {needle:?}) — the RPC-408 heuristic is deleted"
        );
    }

    // @step And it does not read the pending request to classify answers
    assert!(
        !body.contains("get_hitl_request"),
        "send_hitl_response must not read the pending HITL request to \
         classify answers — the wire payload is authoritative"
    );
}

// ============================================================================
// Scenario: New wire shapes round-trip through serde_json
// ============================================================================

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&back, value, "round-trip must preserve the value exactly");
}

fn wire_option(label: &str) -> WireHitlOption {
    WireHitlOption {
        label: label.to_string(),
        description: format!("{label} description"),
    }
}

#[test]
fn new_wire_shapes_round_trip_through_serde_json() {
    // @step Given a wire HitlRequest with 3 questions including one without options and a wire HitlResponse with cancelled false and mixed answers
    let request = WireHitlRequest {
        questions: vec![
            WireHitlQuestion {
                id: "approach".to_string(),
                header: "Approach".to_string(),
                question: "Which approach do you prefer?".to_string(),
                options: vec![wire_option("Option A"), wire_option("Option B")],
            },
            WireHitlQuestion {
                id: "priority".to_string(),
                header: "Priority".to_string(),
                question: "What is the priority?".to_string(),
                options: vec![wire_option("High"), wire_option("Low")],
            },
            WireHitlQuestion {
                id: "notes".to_string(),
                header: "Notes".to_string(),
                question: "Any additional notes?".to_string(),
                options: vec![],
            },
        ],
    };
    let response = WireHitlResponse {
        cancelled: false,
        answers: vec![
            WireHitlAnswer {
                id: "approach".to_string(),
                selected: vec!["Option A".to_string()],
                other: None,
            },
            WireHitlAnswer {
                id: "notes".to_string(),
                selected: vec![],
                other: Some("free text".to_string()),
            },
        ],
    };

    // @step When each value is serialized to JSON and deserialized back
    // @step Then each deserialized value equals the original
    round_trip(&request);
    round_trip(&response);
    assert_eq!(request.questions.len(), 3);
    assert!(request.questions[2].options.is_empty());
    assert!(!response.cancelled);
    assert_eq!(response.answers.len(), 2);

    // @step And a cancelled wire HitlResponse with empty answers also round-trips equal
    let cancelled = WireHitlResponse {
        cancelled: true,
        answers: vec![],
    };
    round_trip(&cancelled);
    assert!(cancelled.cancelled);
    assert!(cancelled.answers.is_empty());
}
