#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compaction-single-source-of-truth.feature
//!
//! CMPCT-026: Eliminate fragile `&& compaction_triggered` guard.
//!
//! These tests cover the `classify_compaction_branch` helper that is the
//! single-source-of-truth classifier for the stream-loop and
//! gemini-continuation compaction-cancel branches.
//!
//! Behaviour matrix (error-side authoritative, flag-side defense-in-depth):
//!
//! | PromptCancelled? | flag | branch                          |
//! |------------------|------|----------------------------------|
//! | yes              | true | Recover { disagreement: None }  |
//! | yes              | false| Recover { disagreement: Some(FlagMissing) } — flag is defensively set |
//! | no               | true | NotCompaction { disagreement: Some(FlagExtraneous) } |
//! | no               | false| NotCompaction { disagreement: None } |

use codelet_cli::interactive::{
    classify_compaction_branch, CompactionBranch, CompactionDisagreement,
};
use codelet_core::TokenState;
use rig::completion::PromptError;
use rig::message::Message;
use std::sync::{Arc, Mutex};

fn token_state_with(compaction_needed: bool) -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed,
    }))
}

fn prompt_cancelled_error() -> anyhow::Error {
    PromptError::PromptCancelled {
        chat_history: Box::new(Vec::<Message>::new()),
    }
    .into()
}

/// Scenario: PromptCancelled with flag true routes to recovery
#[test]
fn prompt_cancelled_with_flag_true_routes_to_recovery() {
    // @step Given the stream has yielded a PromptCancelled error wrapped in the rig StreamingError chain
    let err = prompt_cancelled_error();

    // @step Given the shared token state has compaction_needed set to true
    let token_state = token_state_with(true);

    // @step When the stream loop classifies the error
    let branch = classify_compaction_branch(&err, &token_state);

    // @step Then the loop breaks into the compaction recovery path
    assert!(
        matches!(
            branch,
            CompactionBranch::Recover {
                disagreement: None
            }
        ),
        "PromptCancelled + flag=true must route to Recover with no disagreement, got {branch:?}"
    );

    // @step Then no disagreement warning is logged
    // The classifier returns `disagreement: None` when the two signals agree,
    // which is the single observable contract that callers (stream_loop,
    // gemini_continuation) use to decide whether to emit `tracing::warn!`.
    // Flag must remain true — the classifier never clears.
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "flag should remain true after recovery classification"
    );
}

/// Scenario: PromptCancelled with flag false still recovers
#[test]
fn compaction_cancel_with_flag_false_still_recovers() {
    // @step Given the stream has yielded a PromptCancelled error wrapped in the rig StreamingError chain
    let err = prompt_cancelled_error();

    // @step Given the shared token state has compaction_needed set to false
    let token_state = token_state_with(false);

    // @step When the stream loop classifies the error
    let branch = classify_compaction_branch(&err, &token_state);

    // @step Then the loop breaks into the compaction recovery path
    assert!(
        matches!(
            branch,
            CompactionBranch::Recover {
                disagreement: Some(CompactionDisagreement::FlagMissing)
            }
        ),
        "PromptCancelled + flag=false must route to Recover with FlagMissing disagreement, got {branch:?}"
    );

    // @step Then the compaction_needed flag is set to true as defense-in-depth
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "flag must be defensively set to true when PromptCancelled fires without it"
    );

    // @step Then a warning is emitted that the two signals disagree
    // The `CompactionDisagreement::FlagMissing` marker in the returned branch
    // is the single-source-of-truth signal that the caller uses to invoke
    // `tracing::warn!`. The classifier itself also emits the warn! event, but
    // tests assert on the structured marker to avoid depending on a tracing
    // subscriber in the test harness.
}

/// Scenario: Unrelated error with flag true does not trigger recovery
#[test]
fn unrelated_error_with_flag_true_does_not_trigger_recovery() {
    // @step Given the stream has yielded an error that is NOT a PromptCancelled variant
    let err = anyhow::anyhow!("some unrelated io error");

    // @step Given the shared token state has compaction_needed set to true
    let token_state = token_state_with(true);

    // @step When the stream loop classifies the error
    let branch = classify_compaction_branch(&err, &token_state);

    // @step Then the compaction-cancel branch is not taken
    assert!(
        matches!(
            branch,
            CompactionBranch::NotCompaction {
                disagreement: Some(CompactionDisagreement::FlagExtraneous)
            }
        ),
        "unrelated error + flag=true must route to NotCompaction with FlagExtraneous, got {branch:?}"
    );

    // @step Then a warning is emitted that the flag was set without a PromptCancelled error
    // Marker-as-contract: the `FlagExtraneous` variant is the structural
    // signal that the classifier emitted (or will emit) a tracing::warn!.

    // @step Then the error continues to the normal error classifier cascade
    // NotCompaction means the caller drops through to the subsequent
    // classifier cascade (prompt-too-long, image-content, truncation, etc.).
    // The classifier must NOT mutate the flag in this branch.
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "classifier must NOT mutate the flag when the branch is NotCompaction"
    );
}

/// Negative sanity: no PromptCancelled, no flag — definitely not compaction, no warning.
#[test]
fn unrelated_error_with_flag_false_is_not_compaction() {
    let err = anyhow::anyhow!("network timeout");
    let token_state = token_state_with(false);

    let branch = classify_compaction_branch(&err, &token_state);

    assert!(
        matches!(
            branch,
            CompactionBranch::NotCompaction { disagreement: None }
        ),
        "unrelated error + flag=false must route to NotCompaction with no disagreement, got {branch:?}"
    );
    assert!(
        !token_state.lock().unwrap().compaction_needed,
        "flag must remain false on NotCompaction for an unrelated error"
    );
}
