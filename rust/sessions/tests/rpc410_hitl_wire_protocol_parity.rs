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
//! the trait's `create_session_checked` bridge (Noop hooks, no agent loop,
//! one data directory shared by the whole binary — BUG-397),
//! store a pending internal HITL request on the `BackgroundSession`,
//! block a std thread on `wait_for_hitl_response()`, then drive the
//! handle with new wire `HitlResponse{cancelled, answers}` payloads.
//! Shared helpers live in `rpc410_hitl_wire_protocol_parity_support`;
//! the two static-check scenarios live in
//! rpc410_hitl_wire_protocol_static_checks.rs (REFAC-052).
//!
//! HANG-SAFETY (see rust/sessions/tests/paused_chunk_delivery_rpc409.rs
//! module docs): every scenario that parks a waiter UNCONDITIONALLY
//! sends the unblocking response BEFORE any assertion, and every join
//! is bounded by a hard deadline (`join_waiter`) so a broken send path
//! FAILS the test instead of hanging it. Never assert while a waiter
//! is still parked.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod rpc410_hitl_wire_protocol_parity_support;

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{HitlAnswer as WireHitlAnswer, HitlResponse as WireHitlResponse};
use codelet_sessions::SessionManager;
use codelet_tools::request_user_input::{
    HitlRequest as InternalHitlRequest, HitlResponse as InternalHitlResponse,
};
use rpc410_hitl_wire_protocol_parity_support::{
    freeform_question, fresh_session, join_waiter, option_question, spawn_blocked_waiter,
    three_question_request, yes_no_request,
};

// ============================================================================
// Scenario: A three-question internal request surfaces all questions over
// the wire in order
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_question_request_surfaces_all_questions_in_order() {
    // @step Given a BackgroundSession with a pending internal HITL request containing questions "approach", "priority" and "notes"
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(three_question_request()));

    // @step When the handle's get_hitl_request is called for that session
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let wire = handle
        .get_hitl_request(&sid)
        .expect("get_hitl_request reads the session")
        .expect("a pending request must surface on the wire")
        .payload;

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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(InternalHitlRequest {
        questions: vec![freeform_question("notes", "Notes", "Any additional notes?")],
    }));

    // @step When the handle's get_hitl_request is called for that session
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let wire = handle
        .get_hitl_request(&sid)
        .expect("get_hitl_request reads the session")
        .expect("a pending request must surface on the wire")
        .payload;

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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
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
    let request_id = session
        .pending_question_route()
        .expect("the question is pending")
        .request_id;
    let send_result = handle.send_hitl_response(
        &sid,
        &request_id,
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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
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
    let request_id = session
        .pending_question_route()
        .expect("the question is pending")
        .request_id;
    let send_result = handle.send_hitl_response(
        &sid,
        &request_id,
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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    session.set_hitl_request(Some(yes_no_request()));

    // @step And a thread is blocked on the session's wait_for_hitl_response
    let waiter = spawn_blocked_waiter(&manager, &sid);

    // @step When the handle receives send_hitl_response with an answer for "confirm_choice" with empty selected and other "Yes"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let request_id = session
        .pending_question_route()
        .expect("the question is pending")
        .request_id;
    let send_result = handle.send_hitl_response(
        &sid,
        &request_id,
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
// Scenario: The HITL parity session tests pass when run one after another in
// one process (BUG-397)
// ============================================================================

#[test]
fn session_tests_pass_one_after_another_in_one_process() {
    // @step Given the five HITL parity tests that each create a session
    let session_tests: [fn(); 5] = [
        three_question_request_surfaces_all_questions_in_order,
        question_without_options_surfaces_empty_options_array,
        cancelled_wire_response_reaches_blocked_tool_as_cancelled,
        multi_answer_wire_response_maps_pass_through_keyed_by_answer_id,
        freeform_text_identical_to_option_label_stays_in_other,
    ];

    // @step When they run one after another on one thread in the same test process
    // @step Then every test finds the session it created
    for run in session_tests {
        run();
    }
}
