//! Feature: spec/features/token-accounting-lifecycle.feature

use codelet_cli::interactive::flush_partial_state_before_compaction;
use codelet_cli::interactive_helpers::{
    estimate_message_tokens, pre_compaction_basis, recalculate_token_tracker,
    tracked_pre_compaction_basis,
};
use codelet_core::compaction::{InputUsageObservation, TokenTracker};
use codelet_core::{ApiTokenUsage, StreamingTokenDisplay};
use rig::message::Message;

#[path = "support/bug236_accounting.rs"]
mod support;
use support::{logged, session};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn provenance(tracker: &TokenTracker) -> Result<serde_json::Value, serde_json::Error> {
    Ok(serde_json::to_value(tracker)?["input_accounting"].clone())
}

#[test]
fn fresh_registration_estimates_content_without_an_accounting_warning() -> TestResult {
    // @step Given a fresh session with two messages before inference
    let tracker = TokenTracker::new();
    let messages = vec![
        Message::user("First question"),
        Message::assistant("First response"),
    ];
    // @step When compaction handlers repeatedly capture a token basis
    for _ in 0..2 {
        let (basis, logs) = logged(|| {
            assert_eq!(
                pre_compaction_basis(tracker.input_tokens, &messages),
                estimate_message_tokens(&messages)
            );
            tracked_pre_compaction_basis(&tracker, &messages)
        })?;
        // @step Then the basis equals the content estimate without warnings or provider omission claims
        assert_eq!(basis, estimate_message_tokens(&messages));
        assert!(
            !logs.contains("WARN"),
            "routine registration warned: {logs}"
        );
        assert!(!logs.contains("provider reported no usage"));
    }
    // @step And accounting remains uninitialized
    assert_eq!(provenance(&tracker)?, "uninitialized");
    Ok(())
}

#[test]
fn reported_input_preserves_cache_inclusive_totals_exactly_once() -> TestResult {
    // @step Given a response reports 30000 fresh input and 150000 cache read tokens
    let usage = ApiTokenUsage::new(30_000, 150_000, 0, 500);
    let mut tracker = TokenTracker::new();
    // @step When the tracker records the response and a compaction basis is captured
    let mut observation = InputUsageObservation::default();
    observation.observe(
        &mut tracker,
        &rig::completion::Usage {
            input_tokens: 30_000,
            cache_read_input_tokens: Some(150_000),
            ..Default::default()
        },
    );
    observation.observe(&mut tracker, &rig::completion::Usage::default());
    observation.finish(&mut tracker, &rig::completion::Usage::default());
    assert_eq!(tracker.input_tokens, 180_000);
    tracker.update_from_usage(&usage, 500);
    let (basis, logs) =
        logged(|| tracked_pre_compaction_basis(&tracker, &[Message::user("hello")]))?;
    // @step Then the basis is 180000 with reported provenance and unchanged billing semantics
    assert_eq!(basis, 180_000);
    assert_eq!(provenance(&tracker)?, "reported");
    assert_eq!(tracker.cumulative_billed_input, 30_000);
    assert_eq!(tracker.cumulative_billed_output, 500);
    assert!(!logs.contains("WARN"));
    observation.finish(&mut tracker, &rig::completion::Usage::default());
    assert_eq!(provenance(&tracker)?, "unavailable");
    tracker.update(10, 2, None, None);
    assert_eq!(provenance(&tracker)?, "reported");
    Ok(())
}

#[test]
fn observed_unavailable_input_uses_an_honest_diagnostic_and_estimate() -> TestResult {
    // @step Given a prior response reported input tokens
    let mut tracker = TokenTracker::new();
    tracker.update_from_usage(&ApiTokenUsage::new(200, 300, 0, 10), 10);
    // @step When a completed response has no usable input accounting
    InputUsageObservation::default().finish(&mut tracker, &rig::completion::Usage::default());
    let messages = vec![Message::user("countable content")];
    let (basis, logs) = logged(|| tracked_pre_compaction_basis(&tracker, &messages))?;
    // @step Then the old input is not reused as a current measurement
    assert_eq!(tracker.input_tokens, 0);
    assert_eq!(provenance(&tracker)?, "unavailable");
    assert_eq!(basis, estimate_message_tokens(&messages));
    // @step And the content estimate is accompanied by an unavailable accounting warning without an asserted provider cause
    assert!(
        logs.contains("WARN") && logs.contains("Unavailable"),
        "{logs}"
    );
    assert!(!logs.contains("provider reported no usage"));
    assert_eq!(tracker.cumulative_billed_input, 200);
    tracker.update_display_only(&ApiTokenUsage::new(500, 0, 0, 0), 10);
    assert_eq!(tracker.input_tokens, 0);
    InputUsageObservation::default().finish(
        &mut tracker,
        &rig::completion::Usage {
            cache_read_input_tokens: Some(700),
            cache_creation_input_tokens: Some(50),
            ..Default::default()
        },
    );
    assert_eq!(tracker.input_tokens, 750);
    assert_eq!(provenance(&tracker)?, "reported");
    Ok(())
}

#[test]
fn display_snapshots_and_partial_flushes_do_not_invent_observations() -> TestResult {
    // @step Given a fresh or content estimated tracker and a seeded streaming display
    for estimated in [false, true] {
        let mut session = session();
        session.messages = vec![Message::user("content estimate")];
        if estimated {
            recalculate_token_tracker(&mut session);
        }
        let before = provenance(&session.token_tracker)?;
        let total = session.token_tracker.input_tokens;
        let display = StreamingTokenDisplay::from_cache_inclusive_total(total, 0, 0, 0);
        let usage = ApiTokenUsage::new(total, 0, 0, 0);
        // @step When display state is saved and partial state is flushed before a response
        session.token_tracker.update_display_only(&usage, 0);
        flush_partial_state_before_compaction(&mut session, &mut String::new(), &display)?;
        // @step Then input provenance is unchanged and no missing usage observation is invented
        assert_eq!(provenance(&session.token_tracker)?, before);
        // @step And cache inclusive totals and existing flush billing are preserved
        assert_eq!(session.token_tracker.input_tokens, total);
        assert_eq!(session.token_tracker.cumulative_billed_input, total);
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn restored_accounting_is_not_relabeled_as_provider_reported() -> TestResult {
    use codelet_core::persistence::*;
    // @step Given durable sessions with zero or nonzero saved input accounting
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("restored", directory.path(), "openai")?;
    for saved in [0, 180_000] {
        manifest.token_usage.current_context_tokens = saved;
        let mut session = session();
        session.messages = vec![Message::user("restored nonempty content")];
        // @step When their accounting is restored and a compaction basis is captured
        session.restore_compaction_boundary(&manifest)?;
        let (basis, logs) =
            logged(|| tracked_pre_compaction_basis(&session.token_tracker, &session.messages))?;
        // @step Then saved nonzero totals are preserved and zero totals use content estimates
        assert_eq!(
            basis,
            if saved > 0 {
                saved
            } else {
                estimate_message_tokens(&session.messages)
            }
        );
        // @step And diagnostics identify restored accounting without provider omission warnings
        assert_eq!(provenance(&session.token_tracker)?, "restored");
        assert!(!logs.contains("WARN"), "{logs}");
        assert!(logs.contains("Restored"), "{logs}");
    }
    Ok(())
}

#[test]
fn legacy_accounting_without_provenance_remains_unknown() -> TestResult {
    // @step Given serialized trackers without lifecycle metadata
    for total in [0, 1234] {
        let mut value = serde_json::to_value(TokenTracker::new())?;
        value
            .as_object_mut()
            .ok_or("tracker object")?
            .remove("input_accounting");
        value["input_tokens"] = total.into();
        // @step When trackers with zero or nonzero input are loaded
        let tracker: TokenTracker = serde_json::from_value(value)?;
        let messages = vec![Message::user("legacy content")];
        let (basis, logs) = logged(|| tracked_pre_compaction_basis(&tracker, &messages))?;
        // @step Then provenance is unknown and numeric totals or content estimates remain usable without provider omission warnings
        assert_eq!(provenance(&tracker)?, "unknown");
        assert_eq!(
            basis,
            if total > 0 {
                total
            } else {
                estimate_message_tokens(&messages)
            }
        );
        assert!(!logs.contains("WARN"), "{logs}");
    }
    Ok(())
}

#[test]
fn reset_accounting_does_not_claim_a_missing_provider_response() -> TestResult {
    // @step Given a tracker with previously reported input and billing
    let mut tracker = TokenTracker::new();
    tracker.update_from_usage(
        &ApiTokenUsage::new(1000, 2000, 0, 50).with_reasoning_tokens(7),
        50,
    );
    // @step When accounting is reset after compaction
    tracker.reset_after_compaction();
    // @step Then provenance is reset and no old provider observation is asserted
    assert_eq!(provenance(&tracker)?, "reset");
    assert_eq!(tracker.input_tokens, 3000);
    let messages = vec![Message::user("retained content")];
    let (basis, logs) = logged(|| tracked_pre_compaction_basis(&tracker, &messages))?;
    assert_eq!(basis, 3000);
    assert!(!logs.contains("WARN"));
    // @step And session spend is preserved
    assert_eq!(tracker.cumulative_billed_input, 1000);
    assert_eq!(tracker.cumulative_billed_output, 50);
    assert_eq!(tracker.reasoning_tokens, 7);
    Ok(())
}

#[test]
fn post_compaction_recalculation_is_explicitly_estimated() -> TestResult {
    // @step Given a session with reported input and cache tokens
    let mut session = session();
    session
        .token_tracker
        .update_from_usage(&ApiTokenUsage::new(1000, 2000, 3000, 50), 50);
    // @step When compacted message content replaces the history and accounting is recalculated
    session.messages = vec![Message::user("compact summary")];
    recalculate_token_tracker(&mut session);
    // @step Then the basis equals the new content estimate with estimated provenance
    assert_eq!(provenance(&session.token_tracker)?, "estimated");
    assert_eq!(
        tracked_pre_compaction_basis(&session.token_tracker, &session.messages),
        estimate_message_tokens(&session.messages)
    );
    // @step And stale cache fields are cleared without changing session spend
    assert_eq!(session.token_tracker.cache_read_input_tokens, None);
    assert_eq!(session.token_tracker.cache_creation_input_tokens, None);
    assert_eq!(session.token_tracker.cumulative_billed_input, 1000);
    assert_eq!(session.token_tracker.cumulative_billed_output, 50);
    Ok(())
}

#[test]
fn empty_content_legitimately_has_a_zero_basis() -> TestResult {
    // @step Given uninitialized or unavailable accounting with empty message content
    for unavailable in [false, true] {
        let mut tracker = TokenTracker::new();
        if unavailable {
            tracker.observe_input_usage(&ApiTokenUsage::default());
        }
        for messages in [vec![], vec![Message::user("")]] {
            // @step When a compaction basis is captured
            let (basis, logs) = logged(|| tracked_pre_compaction_basis(&tracker, &messages))?;
            // @step Then the basis is zero without warnings or invented tokens
            assert_eq!(basis, 0);
            assert!(!logs.contains("WARN"));
        }
    }
    Ok(())
}

#[test]
fn selection_reset_invalidates_old_input_evidence() -> TestResult {
    // @step Given a session with reported input for its previous selection
    let mut session = session();
    session.messages = vec![Message::user("retained across model selection")];
    session
        .token_tracker
        .update_from_usage(&ApiTokenUsage::new(1000, 0, 0, 10), 10);
    // @step When provider state is reset for a model or provider selection change
    session.reset_provider_state()?;
    // @step Then old input evidence is invalidated without claiming a provider omission
    assert_eq!(provenance(&session.token_tracker)?, "reset");
    assert_eq!(session.token_tracker.input_tokens, 0);
    // @step And retained content remains measurable by estimation
    let (basis, logs) =
        logged(|| tracked_pre_compaction_basis(&session.token_tracker, &session.messages))?;
    assert_eq!(basis, estimate_message_tokens(&session.messages));
    assert!(!logs.contains("WARN"));
    Ok(())
}
