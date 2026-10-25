//! Feature: spec/features/codex-typed-protocol-continuation.feature
#[path = "support/prov162_authority.rs"]
mod authority;
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "fixtures/prov148.rs"]
mod fixture;
#[path = "support/prov162.rs"]
mod support;

use codelet_providers::codex::history::CodexHistory;
use environment::{Environment, TestResult};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::Message,
};
use serde_json::json;
use support::{collect, model, output, requests, terminal};

#[tokio::test]
#[serial_test::serial]
async fn known_typed_output_continues_with_exact_owned_items() -> TestResult {
    // @step Given a local Codex response contains owned reasoning text and negotiated completed hosted search
    let _environment = Environment::new()?;
    for incomplete in [false, true] {
        for completion in [false, true] {
            let owned = support::known_output(incomplete, completion);
            let server = fixture::Server::start(
                vec![
                    terminal(
                        owned.clone(),
                        if incomplete {
                            "incomplete"
                        } else {
                            "completed"
                        },
                        false,
                        incomplete.then_some("interrupted"),
                    ),
                    terminal(
                        vec![support::message("final", "final_answer")],
                        "completed",
                        true,
                        None,
                    ),
                ],
                false,
            )
            .await?;
            let history = CodexHistory::default();
            let model = model(&server, history.clone())?;
            // @step When eligible completed or interrupted responses continue through streaming or completion entry points
            if completion {
                let response = model
                    .clone()
                    .with_streaming_completions()
                    .completion(support::request(&model))
                    .await?;
                assert_eq!(response.usage.total_tokens, 40);
            } else {
                let observed = collect(&model).await?;
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!(observed.finals, [40]);
                assert_eq!(observed.usage, [20, 20]);
                assert_eq!(observed.records.len(), 2);
                assert_ne!(observed.records[0].0, observed.records[1].0);
            }
            // @step Then the next physical request preserves every validated native item and opaque byte in order
            let sent = requests(&server)?;
            assert_eq!(sent.len(), 2);
            assert_eq!(
                &sent[1]["input"].as_array().ok_or("missing input")?[1..],
                owned.as_slice()
            );
            // @step And each physical usage is billed once with one final aggregate
            assert!(history.snapshot()?.is_some());
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn refusal_and_unsafe_terminals_never_trigger_protocol_replay() -> TestResult {
    // @step Given local Codex responses contain refusal filtering budget failure or incomplete client calls
    let _environment = Environment::new()?;
    for kind in support::UNSAFE_KINDS {
        let server = fixture::Server::start(vec![support::unsafe_terminal(kind)], false).await?;
        // @step When explicit turn markers otherwise permit continuation
        let observed = collect(&model(&server, CodexHistory::default())?).await?;
        // @step Then refusal remains visible and unsafe output never sends a follow up or executable tool
        assert_eq!(requests(&server)?.len(), 1);
        assert_eq!(observed.tools, 0);
        if kind == "refusal" {
            assert!(observed.errors.is_empty(), "{observed:?}");
            assert!(observed.text.contains("Cannot comply"));
            assert_eq!(observed.finals, [20]);
        } else {
            assert_eq!(observed.errors.len(), 1, "{observed:?}");
            assert!(observed.finals.is_empty());
        }
        // @step And measured terminal usage is retained once without unsafe final completion
        assert_eq!(observed.usage, [20]);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn empty_unknown_and_malformed_continuation_output_fail_closed() -> TestResult {
    // @step Given eligible local Codex terminals contain empty unknown or malformed typed output
    let _environment = Environment::new()?;
    for (case, owned) in support::malformed_output().into_iter().enumerate() {
        let server = fixture::Server::start(
            vec![
                terminal(owned, "completed", false, None),
                terminal(
                    vec![support::message("final", "final_answer")],
                    "completed",
                    true,
                    None,
                ),
            ],
            false,
        )
        .await?;
        let history = CodexHistory::default();
        // @step When the real model consumes those terminals
        let observed = collect(&model(&server, history.clone())?).await?;
        // @step Then no follow up or executable tool is sent and no checkpoint is published
        assert!(history.snapshot()?.is_none(), "malformed case {case}");
        // @step And each trustworthy terminal usage remains observable exactly once
        support::assert_failure(&observed, &server, 1)?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn protocol_limits_stay_finite_across_transport_replacement() -> TestResult {
    // @step Given an interrupted transport precedes an indefinitely continuing typed Codex response chain
    let _environment = Environment::new()?;
    let mut bodies = vec![fixture::text()];
    bodies.extend(vec![terminal(output(), "completed", false, None); 4]);
    let server = fixture::Server::start(bodies, false).await?;
    // @step When the existing recovery policy replaces the transport
    let observed = collect(&model(&server, CodexHistory::default())?).await?;
    // @step Then only three protocol follow ups occur and exhaustion identifies the protocol budget
    assert_eq!(requests(&server)?.len(), 5);
    assert_eq!(observed.aborted, 1);
    assert_eq!(observed.errors.len(), 1, "{observed:?}");
    assert!(
        observed.errors[0].contains("protocol continuation limit")
            && observed.errors[0].contains("/continue")
    );
    // @step And each settled physical response is billed once without final completion
    assert_eq!(observed.usage, [20; 4]);
    assert!(observed.finals.is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn admission_and_cancellation_cannot_publish_partial_typed_lineage() -> TestResult {
    // @step Given an admitted Codex response grows typed continuation beyond capacity or reaches caller cancellation
    let _environment = Environment::new()?;
    for cancel in [false, true] {
        let mut owned = output();
        if !cancel {
            owned[0]["encrypted_content"] = json!("oversized owned content ".repeat(2000));
        }
        let server =
            fixture::Server::start(vec![terminal(owned, "completed", false, None)], false).await?;
        let history = CodexHistory::default();
        let mut model = model(&server, history.clone())?;
        if !cancel {
            model = model.with_request_admission(
                codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: Some(3000),
                    output_reservation: Some(100),
                },
            );
        }
        // @step When the next physical request would begin
        if cancel {
            let stream = support::stop_at_usage(&model).await?;
            drop(stream);
        } else {
            let observed = collect(&model).await?;
            support::assert_failure(&observed, &server, 1)?;
            assert!(
                observed.errors[0].contains("admission") || observed.errors[0].contains("capacity"),
                "{observed:?}"
            );
        }
        // @step Then no forbidden request is sent and the committed checkpoint remains unchanged
        assert_eq!(requests(&server)?.len(), 1);
        assert!(history.snapshot()?.is_none());
        // @step And already measured physical usage is retained once
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn generation_and_account_changes_invalidate_protocol_authority() -> TestResult {
    // @step Given a measured typed Codex response is waiting to continue
    let _environment = Environment::new()?;
    authority::assert_in_flight_reset_bills_without_publication().await?;
    for account in [false, true] {
        let server =
            fixture::Server::start(vec![terminal(output(), "completed", false, None)], false)
                .await?;
        let history = CodexHistory::default();
        let model = if account {
            support::authenticated_model(&server, history.clone())?
        } else {
            model(&server, history.clone())?
        };
        let mut stream = support::stop_at_usage(&model).await?;
        // @step When its live generation or stored synthetic account changes
        if account {
            support::change_account()?;
        } else {
            history.transition_generation(1)?;
        }
        let previous = history.snapshot()?;
        // @step Then existing send time authority rejects the follow up before inference
        support::assert_authority_errors(&mut stream, account).await?;
        assert_eq!(requests(&server)?.len(), 1);
        // @step And no stale typed checkpoint replaces the current owner
        assert_eq!(history.snapshot()?, previous);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn genuine_follow_ups_preserve_typed_ancestry_and_reject_rewrites() -> TestResult {
    // @step Given a successful typed protocol chain has committed its exact owned history
    let _environment = Environment::new()?;
    let server = fixture::Server::start(
        vec![
            terminal(output(), "completed", false, None),
            terminal(
                vec![support::message("final", "final_answer")],
                "completed",
                true,
                None,
            ),
            terminal(
                vec![support::message("next", "final_answer")],
                "completed",
                true,
                None,
            ),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let model = model(&server, history.clone())?;
    let observed = collect(&model).await?;
    assert!(observed.errors.is_empty(), "{observed:?}");
    // @step When a genuine user follow up or changed model or rewritten assistant history is submitted
    let previous = history.snapshot()?.ok_or("missing typed history")?;
    let request = CompletionRequestBuilder::new(model.clone(), "next genuine task")
        .preamble("Test instructions".into())
        .messages(vec![
            Message::user("Hello"),
            Message::assistant("partialfinal"),
        ])
        .build();
    let mut next = model.stream(request).await?;
    while let Some(event) = next.next().await {
        event?;
    }
    for changed in [false, true] {
        let mut request = json!({"model":if changed {"changed-model"} else {"shared-model"},"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"rewritten ancestry"}]}]});
        assert!(history.prepare(&mut request).is_err());
    }
    // @step Then the compatible user request retains exact native ancestry and incompatible requests send nothing
    let sent = requests(&server)?;
    assert_eq!(sent.len(), 3);
    let owned = previous
        .items
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        &sent[2]["input"].as_array().ok_or("missing input")?[..owned.len()],
        owned.as_slice()
    );
    Ok(())
}
