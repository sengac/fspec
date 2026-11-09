//! BUG-403 — HITL wire payloads require their options and answers, and the
//! RPC-410 send-path static check requires its end marker.
//!
//! Feature: spec/features/hitl-wire-required-fields.feature

use codelet_rpc_types::{HitlQuestion, HitlResponse};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_hitl_question_without_options_is_rejected() {
    // @step Given a HITL question payload with no options field
    let payload = json!({"id": "q1", "header": "Pick", "question": "Which?"});
    // @step When it is deserialized
    let parsed = serde_json::from_value::<HitlQuestion>(payload);
    // @step Then deserialization fails
    assert!(parsed.is_err(), "{parsed:?}");
}

#[test]
fn a_hitl_response_without_answers_is_rejected() -> TestResult {
    // @step Given a submitted HITL response payload with no answers field
    let payload = json!({"cancelled": false});
    // @step When it is deserialized
    let parsed = serde_json::from_value::<HitlResponse>(payload);
    // @step Then deserialization fails and a response with an explicit empty answers list still parses
    assert!(parsed.is_err(), "{parsed:?}");
    let cancelled: HitlResponse =
        serde_json::from_value(json!({"cancelled": true, "answers": []}))?;
    assert!(cancelled.cancelled && cancelled.answers.is_empty());
    Ok(())
}

#[test]
fn the_send_path_static_check_requires_its_end_marker() -> TestResult {
    // @step Given the RPC-410 send-path static check
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs");
    let source = std::fs::read_to_string(path)?;
    // @step When it locates the end of the send_hitl_response body
    let locates_end = source.contains(".find(\"fn get_pause_state(\")");
    // @step Then a missing end marker fails the check instead of falling back to the rest of the source
    assert!(locates_end, "the check no longer locates its end marker");
    assert!(
        !source.contains("unwrap_or(handle_src.len())"),
        "the check still falls back to the rest of the source"
    );
    assert!(
        source.contains("must define get_pause_state after send_hitl_response"),
        "the check must fail naming the missing end marker"
    );
    Ok(())
}
