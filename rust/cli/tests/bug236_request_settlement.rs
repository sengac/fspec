//! Feature: spec/features/token-accounting-request-settlement.feature

#[path = "support/bug236_scripted_stream.rs"]
mod fixture;

use codelet_cli::interactive_helpers::{
    estimate_message_tokens, recalculate_token_tracker, tracked_pre_compaction_basis,
};
use codelet_core::compaction::InputAccounting;
use codelet_core::ApiTokenUsage;
use codelet_providers::ProviderType;
use fixture::{aborted, final_reply, run, streamed, text, tool, unreported_reply, TestResult};
use rig::message::Message;

#[tokio::test]
#[serial_test::serial]
async fn streamed_input_survives_zero_input_final_output_settlement() -> TestResult {
    // @step Given a response streams 30000 fresh input and 150000 cached input tokens
    let script = vec![streamed(), text("accepted answer"), final_reply(0, 17)];
    // @step When its final response reports zero input and positive output and the turn settles
    let session = run(ProviderType::OpenAI, vec![script]).await?;
    // @step Then current input remains 180000 with reported provenance and the original cache split
    assert_eq!(session.token_tracker.input_tokens, 180_000);
    assert_eq!(
        session.token_tracker.input_accounting,
        InputAccounting::Reported
    );
    assert_eq!(session.token_tracker.cache_read_input_tokens, Some(150_000));
    // @step And output and reasoning reflect the final display without inventing input billing
    assert_eq!(session.token_tracker.output_tokens, 17);
    assert_eq!(session.token_tracker.reasoning_tokens, 3);
    assert_eq!(session.token_tracker.cumulative_billed_input, 0);
    Ok(())
}

#[test]
fn post_compaction_input_survives_stale_display_settlement() -> TestResult {
    // @step Given content recalculation replaced a reported cached context and reset its cache split
    for reset in [false, true] {
        let mut session = fixture::session(ProviderType::OpenAI)?;
        session
            .token_tracker
            .update_from_usage(&ApiTokenUsage::new(30_000, 150_000, 0, 10), 10);
        session.messages = vec![Message::user("compact content")];
        recalculate_token_tracker(&mut session);
        if reset {
            session.token_tracker.reset_after_compaction();
        }
        let expected = session.token_tracker.clone();
        // @step When the old streaming display settles after compaction
        session
            .token_tracker
            .update_display_only(&ApiTokenUsage::new(30_000, 150_000, 0, 17), 17);
        // @step Then the recalculated input estimate and estimated or reset provenance are preserved
        assert_eq!(
            session.token_tracker.input_tokens,
            estimate_message_tokens(&session.messages)
        );
        assert_eq!(
            session.token_tracker.input_accounting,
            expected.input_accounting
        );
        // @step And stale cache tokens never reappear and session spend is unchanged
        assert_eq!(session.token_tracker.cache_read_input_tokens, None);
        assert_eq!(session.token_tracker.cache_creation_input_tokens, None);
        assert_eq!(
            session.token_tracker.cumulative_billed_input,
            expected.cumulative_billed_input
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn aborted_attempt_usage_cannot_supply_a_replacement_response() -> TestResult {
    // @step Given attempt A streams input usage and is aborted before completion
    let script = aborted();
    // @step When replacement attempt B completes without usable input usage
    let session = run(ProviderType::OpenAI, vec![script]).await?;
    // @step Then input accounting is unavailable rather than inherited from attempt A
    assert_eq!(
        session.token_tracker.input_accounting,
        InputAccounting::Unavailable
    );
    assert_eq!(session.token_tracker.input_tokens, 0);
    // @step And the current content estimate remains usable without a provider omission claim
    assert_eq!(
        tracked_pre_compaction_basis(&session.token_tracker, &session.messages),
        estimate_message_tokens(&session.messages)
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn internal_tool_followup_starts_independent_input_accounting() -> TestResult {
    for absent in [false, true] {
        for provider in [ProviderType::OpenAI, ProviderType::Gemini] {
            // @step Given one physical request streams input usage and completes a tool call
            let first = vec![streamed(), tool(), final_reply(180_000, 1)];
            // @step When its internal followup request completes without input usage
            let final_response = if absent {
                unreported_reply()
            } else {
                final_reply(0, 17)
            };
            let session = run(
                provider,
                vec![first, vec![text("tool answer"), final_response]],
            )
            .await?;
            // @step Then the followup accounting is unavailable rather than reusing the tool request input
            assert_eq!(
                session.token_tracker.input_accounting,
                InputAccounting::Unavailable
            );
            assert_eq!(session.token_tracker.input_tokens, 0);
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn retry_after_a_failed_request_discards_provisional_input() -> TestResult {
    // @step Given a physical request streams input usage and fails with a retryable connection error
    let first = vec![
        streamed(),
        Err(rig::completion::CompletionError::ProviderError(
            "connection reset by peer".into(),
        )),
    ];
    // @step When its retry completes without input usage
    let session = run(
        ProviderType::OpenAI,
        vec![first, vec![text("retry answer"), final_reply(0, 17)]],
    )
    .await?;
    // @step Then retry accounting is unavailable rather than reusing failed request input
    assert_eq!(
        session.token_tracker.input_accounting,
        InputAccounting::Unavailable
    );
    assert_eq!(session.token_tracker.input_tokens, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn gemini_continuation_attempts_discard_aborted_input_observations() -> TestResult {
    for absent in [false, true] {
        // @step Given a Gemini empty tool followup starts a continuation stream
        let first = vec![tool(), final_reply(0, 1)];
        let empty = vec![text(""), final_reply(0, 1)];
        // @step When continuation attempt A reports input then aborts and replacement B completes without input usage
        let mut continuation = aborted();
        if absent {
            continuation.pop();
            continuation.push(unreported_reply());
        }
        let session = run(ProviderType::Gemini, vec![first, empty, continuation]).await?;
        // @step Then settled Gemini input is unavailable rather than inherited from the aborted continuation
        assert_eq!(
            session.token_tracker.input_accounting,
            InputAccounting::Unavailable
        );
        assert_eq!(session.token_tracker.input_tokens, 0);
    }
    Ok(())
}
