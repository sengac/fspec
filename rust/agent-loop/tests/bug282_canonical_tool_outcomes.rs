//! Feature: spec/features/canonical-streamed-tool-outcomes.feature

#[path = "support/bug282.rs"]
pub mod fixture;
#[path = "support/bug282_negative.rs"]
pub mod negative;
#[path = "../../sessions/src/replay_content.rs"]
pub mod replay_content;
#[path = "../../sessions/src/replay_media.rs"]
pub mod replay_media;
#[path = "../../sessions/src/replay_transcript.rs"]
pub mod replay_transcript;
use codelet_core::persistence::*;
use fixture::{history, payload, string_payload, Fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn scenario_streamed_textual_outcomes_retain_exact_canonical_bytes() -> TestResult {
    // @step Given real tools return canonical JSON strings, object escapes and Windows paths
    for returned in [payload(), string_payload()] {
        let fixture = Fixture::new()?;
        // @step When their streamed outcomes are persisted by the native output consumer
        let server = fixture.run_output(1, true, returned.clone()).await?;
        // @step Then the event, runtime, durable originals and checkpoint retain identical outcome text
        let expected = returned.to_string();
        if returned.is_string() {
            assert!(expected.starts_with('"') && expected.ends_with('"'));
            assert!(serde_json::from_str::<serde_json::Value>(&expected)?.is_string());
        }
        let session = fixture.background.inner.lock().await;
        let checkpoint = serde_json::to_value(history(&session)?.snapshot()?)?;
        let outputs: Vec<_> = checkpoint["items"]
            .as_array()
            .ok_or("items")?
            .iter()
            .filter(|item| item["type"] == "function_call_output")
            .collect();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0]["output"], expected);
        assert_eq!(
            fixture
                .output
                .results
                .lock()
                .map_err(|_| "capture")?
                .as_slice(),
            std::slice::from_ref(&expected)
        );
        let envelopes = get_session_message_envelopes(fixture.background.id)?;
        let replay = replay_content::Replay::parse(&envelopes)?;
        assert!(codelet_cli::session::replay::same_replay_messages(
            &replay.messages,
            &session.messages
        ));
        let originals = fixture.originals()?;
        assert!(originals
            .as_array()
            .ok_or("originals")?
            .iter()
            .any(|original| original["metadata"]["message"]["content"][0]["content"] == expected));
        let requests = server.requests.lock().map_err(|_| "requests")?;
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[1]["input"]
                .as_array()
                .ok_or("input")?
                .iter()
                .find(|item| item["type"] == "function_call_output")
                .ok_or("outcome")?["output"],
            expected
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_authentic_streamed_outcomes_survive_successive_compaction_and_cold_continuation(
) -> TestResult {
    // @step Given genuine streamed outcomes and a committed Codex checkpoint
    let fixture = Fixture::new()?;
    for generation in 1..=2 {
        let source = fixture
            .background
            .inner
            .lock()
            .await
            .capture_durable_compaction(fixture.background.id)?;
        let _server = fixture.run(generation, true).await?;
        let originals = fixture.originals()?;
        let dag = format!("<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Work\">original task; authentic work retained</dag-node>", source.prefix.len()-1);
        let mut published = None;
        // @step When first and successive admitted compactions publish and restore cold
        let committed = fixture
            .background
            .inner
            .lock()
            .await
            .commit_selected_compaction(source, uuid::Uuid::new_v4(), &dag, |boundary| {
                assert_eq!(
                    load_session(boundary.identity.session_id)?
                        .compaction_commit
                        .as_ref(),
                    Some(boundary)
                );
                published = Some(boundary.identity.clone());
                Ok(())
            })?;
        let boundary = committed.compaction_commit.as_ref().ok_or("boundary")?;
        assert_eq!(published.as_ref(), Some(&boundary.identity));
        assert_eq!(boundary.identity.generation, generation as u64);
        assert_eq!(
            boundary.checkpoint.payload["_codexCompaction"]["durableGeneration"],
            generation
        );
        assert!(!boundary.publication_pending);
        fixture.cold().await?;
        // @step Then real continuation retains original outcomes exactly once without repeating effects
        let continuation = fixture.run(generation + 10, false).await?;
        {
            let requests = continuation.requests.lock().map_err(|_| "requests")?;
            assert_eq!(requests.len(), 1);
            let outcomes: Vec<_> = requests[0]["input"]
                .as_array()
                .ok_or("input")?
                .iter()
                .filter(|item| item["type"] == "function_call_output")
                .collect();
            assert_eq!(outcomes.len(), 1);
            assert_eq!(outcomes[0]["call_id"], format!("call-{generation}"));
            assert_eq!(outcomes[0]["output"], payload().to_string());
        }
        assert_eq!(
            std::fs::read_to_string(
                fixture
                    .directory
                    .path()
                    .join(format!("effect-{generation}"))
            )?,
            "effect once"
        );
        assert!(!fixture
            .directory
            .path()
            .join(format!("effect-{}", generation + 10))
            .exists());
        // @step And original history bytes and the advancing checkpoint and DAG bindings are preserved
        let after = fixture.originals()?;
        assert_eq!(
            &after.as_array().ok_or("after")?[..originals.as_array().ok_or("before")?.len()],
            originals.as_array().ok_or("before")?
        );
        assert_eq!(
            load_session(fixture.background.id)?
                .compaction
                .as_ref()
                .ok_or("DAG")?
                .summary,
            dag
        );
        let session = fixture.background.inner.lock().await;
        assert_eq!(history(&session)?.generation()?, generation as u64);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_altered_streamed_ancestry_still_blocks_atomic_compaction() -> TestResult {
    // @step Given genuine streamed outcomes and a committed Codex checkpoint
    for change in negative::Change::ALL {
        let (fixture, source) = negative::setup(change).await?;
        let all_originals = fixture.originals()?;
        // @step When an outcome, invocation, assistant text or DAG ancestor is changed or missing
        negative::mutate(&fixture, change).await?;
        let mut session = fixture.background.inner.lock().await;
        let before = serde_json::to_value(load_session(fixture.background.id)?)?;
        let originals = fixture.originals()?;
        let checkpoint = history(&session)?.snapshot()?;
        let mut published = false;
        let dag = format!(
            "<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Work\">original task</dag-node>",
            source.prefix.len() - 1
        );
        // @step Then selected compaction refuses publication and preserves the manifest and checkpoint
        let failure = session
            .commit_selected_compaction(source, uuid::Uuid::new_v4(), &dag, |_| {
                published = true;
                Ok(())
            })
            .err()
            .ok_or("altered ancestry accepted")?;
        assert!(failure.promoted_identity().is_none());
        negative::assert_rejection(&failure, change);
        assert!(!published);
        assert_eq!(
            serde_json::to_value(load_session(fixture.background.id)?)?,
            before
        );
        assert_eq!(history(&session)?.snapshot()?, checkpoint);
        assert_eq!(fixture.originals()?, originals);
        negative::assert_records(&all_originals)?;
        // @step And inference remains blocked without fallback or fabricated outcomes
        assert!(session.ensure_compaction_inference_ready().is_err());
    }
    Ok(())
}
