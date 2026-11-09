//! BUG-546: a steered Codex request is replayed only when every call of the
//! checkpointed response has an outcome.
//!
//! Feature: spec/features/interrupt-steer-and-queue-input.feature
//!
//! The steered request below has the shape the CLI stream loop builds from
//! session history: each call is followed by its result, then the steer. The
//! guard that refuses a call without an outcome stays; the stream loop fix
//! (deliver the steer only at the step boundary) is what supplies every
//! result.

use codelet_providers::codex::history::{CheckpointError, CodexHistory};
use rig::completion::CompletionError;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Scenario: Codex replays a steered request only when every call has its outcome
#[test]
fn codex_replays_a_steered_request_only_when_every_call_has_its_outcome() -> TestResult {
    // @step Given a Codex checkpoint whose response requested the calls "call_1" and "call_2"
    let history = CodexHistory::default();
    let user = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"what is this project about?"}]});
    let calls = [
        json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"Bash","arguments":"{\"command\":\"sed -n '1,240p' README.md\"}"}),
        json!({"type":"function_call","id":"fc_2","call_id":"call_2","name":"Bash","arguments":"{\"command\":\"sed -n '1,240p' spec/FOUNDATION.md\"}"}),
    ];
    history.commit(&json!({"model":"test","input":[user]}), &calls)?;
    let saved = history.snapshot()?;
    let first =
        json!({"type":"function_call_output","call_id":"call_1","output":"README contents"});
    let second =
        json!({"type":"function_call_output","call_id":"call_2","output":"FOUNDATION contents"});
    let steer = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"use the staging database"}]});

    // @step When a steered request carries only the result of "call_1"
    let mut early = json!({"model":"test","input":[user, calls[0], first, steer]});
    let unchanged = early.clone();
    let error = history
        .prepare(&mut early)
        .err()
        .ok_or("a call without an outcome was replayed")?;

    // @step Then the request is refused because "call_2" has no recorded outcome
    assert!(matches!(error, CompletionError::RequestError(ref source)
        if matches!(source.downcast_ref::<CheckpointError>(), Some(CheckpointError::MissingOutcome(call)) if call == "call_2")));
    assert_eq!(early, unchanged, "a refused request is not rewritten");
    assert_eq!(
        history.snapshot()?,
        saved,
        "a refused request keeps the checkpoint"
    );

    // @step When the steered request carries the results of "call_1" and "call_2" before the steer
    let mut boundary =
        json!({"model":"test","input":[user, calls[0], first, calls[1], second, steer]});
    history.prepare(&mut boundary)?;

    // @step Then the request is accepted with an output for every call
    let input = boundary["input"].as_array().ok_or("input missing")?;
    for call_id in ["call_1", "call_2"] {
        assert_eq!(
            input
                .iter()
                .filter(|item| item["type"] == "function_call_output" && item["call_id"] == call_id)
                .count(),
            1,
            "{call_id} carries exactly one output: {input:?}"
        );
    }
    assert_eq!(input.last(), Some(&steer), "the steer follows every output");
    Ok(())
}
