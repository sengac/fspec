//! Feature: spec/features/compaction-checkpoint-settlement.feature

#[path = "support/bug242_assertions.rs"]
mod assertions;
#[path = "support/bug242_codex.rs"]
mod codex;
#[path = "support/bug234_timing.rs"]
pub mod timing;

use codelet_core::persistence::load_session;
use codelet_providers::codex::history::CodexHistory;
use codelet_rpc_types::StreamChunk;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_blocked_first_checkpointed_call_permits_truthful_unknown_outcome_synthesis(
) -> codex::TestResult {
    // @step Given an actual Codex compactor checkpoints a first call that starts but produces no result
    let harness = timing::Harness::new().await?;
    codex::mount(&harness.server, harness.session.id, false, true).await;
    let history = CodexHistory::default();
    let gate = codex::Gate::default();
    let model = codex::model(&harness.server, history.clone())?;
    let agent = codex::agent(model, harness.session.id, gate.clone());
    // @step When exploration expires while that call is still executing
    let running = codex::run(agent, harness.session.id);
    let boundary = async {
        tokio::time::timeout(Duration::from_secs(1), gate.started.notified()).await?;
        Ok::<_, Box<dyn std::error::Error>>(history.snapshot()?)
    };
    let (result, checkpoint) = tokio::join!(running, boundary);
    let settled = history.snapshot()?;
    gate.release.notify_one();
    tokio::time::timeout(Duration::from_secs(1), gate.finished.notified()).await?;
    // @step Then tool-free synthesis receives explicit outcome unknown records with native call identities
    assert_eq!(result?, codex::DAG);
    assertions::synthesis(&harness.bodies().await, &["blocked-call", "unstarted-call"])?;
    // @step And native lineage survives without claiming success cancellation or non-execution
    assert!(settled
        .as_ref()
        .ok_or("settled checkpoint")?
        .items
        .starts_with(&checkpoint?.ok_or("initial checkpoint")?.items));
    assert_eq!(history.generation()?, 0);
    // @step And late actor completion does not rewrite the settled handoff or repeat execution
    assert_eq!(history.snapshot()?, settled);
    assert_eq!(gate.executions.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_partial_batches_preserve_genuine_evidence_without_inventing_later_outcomes(
) -> codex::TestResult {
    // @step Given an actual Codex compactor reads originals before a blocked call and an unstarted later call
    let harness = timing::Harness::new().await?;
    codex::originals(harness.session.id)?;
    codex::mount(&harness.server, harness.session.id, true, true).await;
    let history = CodexHistory::default();
    let gate = codex::Gate::default();
    let model = codex::model(&harness.server, history.clone())?;
    let agent = codex::agent(model.clone(), harness.session.id, gate.clone());
    // @step When exploration expires inside that checkpointed batch
    let (result, started) = tokio::join!(
        codex::run(agent, harness.session.id),
        tokio::time::timeout(Duration::from_secs(1), gate.started.notified())
    );
    started?;
    gate.release.notify_one();
    tokio::time::timeout(Duration::from_secs(1), gate.finished.notified()).await?;
    // @step Then tool-free synthesis marks both unresolved calls outcome unknown without claiming cancellation or success
    assert_eq!(result?, codex::DAG);
    assertions::synthesis(&harness.bodies().await, &["blocked-call", "unstarted-call"])?;
    // @step And the completed originals result survives exactly once with authentic identities
    let saved = serde_json::to_value(history.snapshot()?.ok_or("checkpoint")?)?;
    let outputs: Vec<_> = saved["items"]
        .as_array()
        .ok_or("native items")?
        .iter()
        .filter(|item| item["type"] == "function_call_output" && item["call_id"] == "original-call")
        .collect();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["call_id"], "original-call");
    for evidence in ["committed evidence", "committed response"] {
        assert!(outputs[0]["output"].to_string().contains(evidence));
    }
    {
        use rig::completion::CompletionModel;
        let genuine = rig::message::Message::tool_result_with_call_id(
            "original-item",
            Some("original-call".into()),
            outputs[0]["output"].as_str().ok_or("genuine result text")?,
        );
        assert!(model.settle_tool_results(&[genuine])?.is_empty());
        assert_eq!(
            serde_json::to_value(history.snapshot()?.ok_or("checkpoint")?)?,
            saved
        );
    }
    for item_id in ["original-item", "wrong-item"] {
        use rig::completion::CompletionModel;
        let replacement = rig::message::Message::tool_result_with_call_id(
            item_id,
            Some("original-call".into()),
            "not the genuine originals result",
        );
        assert!(model.settle_tool_results(&[replacement]).is_err());
        assert_eq!(
            serde_json::to_value(history.snapshot()?.ok_or("checkpoint")?)?,
            saved
        );
    }
    // @step And admitted synthesis preserves the selected model reservation generation and absolute deadline without repeated execution
    assert_eq!(history.generation()?, 0);
    assert_eq!(harness.bodies().await.len(), 2);
    assert_eq!(gate.executions.load(Ordering::SeqCst), 1);
    codelet_tools::set_session_search_handler(harness.session.id, None);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_completed_checkpoint_evidence_crosses_the_synthesis_barrier() -> codex::TestResult
{
    // @step Given an actual Codex compactor completes its originals call before its next response stalls
    let harness = timing::Harness::new().await?;
    codex::originals(harness.session.id)?;
    let continued = codex::mount(&harness.server, harness.session.id, false, false).await;
    let history = CodexHistory::default();
    let agent = codex::agent(
        codex::model(&harness.server, history.clone())?,
        harness.session.id,
        codex::Gate::default(),
    );
    // @step When exploration expires with no unresolved tool outcome
    let (result, boundary) = tokio::join!(
        codex::run(agent, harness.session.id),
        tokio::time::timeout(Duration::from_secs(1), continued.notified())
    );
    boundary?;
    assert_eq!(result?, codex::DAG);
    // @step Then physical tool-free synthesis retains the original result and native reasoning exactly once
    let bodies = harness.bodies().await;
    let synthesis = bodies
        .iter()
        .find(|body| !timing::has_tools(body))
        .ok_or("physical synthesis")?;
    let input = synthesis["input"].as_array().ok_or("input")?;
    let outputs: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .collect();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["call_id"], "original-call");
    assert!(outputs[0]["output"]
        .to_string()
        .contains("committed evidence"));
    assert_eq!(
        input
            .iter()
            .filter(|item| item["id"] == "native-reasoning")
            .count(),
        1
    );
    assert!(input
        .iter()
        .any(|item| item["encrypted_content"] == "preserve-native-ciphertext"));
    // @step And the selected model output reservation and checkpoint generation remain unchanged
    assert_eq!(synthesis["model"], "selected-codex-model");
    assert_eq!(synthesis["max_output_tokens"], 4096);
    assert_eq!(history.generation()?, 0);
    for body in bodies {
        codelet_providers::request_admission::admit_complete_request(
            &serde_json::to_vec(&body)?,
            codex::LIMITS,
        )?;
    }
    codelet_tools::set_session_search_handler(harness.session.id, None);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_both_publication_routes_preserve_the_prior_dag_on_invalid_native_lineage(
) -> codex::TestResult {
    let _budget = codex::Budget::install();
    // @step Given registered recovery and direct generation have a previously committed DAG
    for generated in [false, true] {
        let harness = timing::Harness::new().await?;
        harness.prior_generation().await?;
        let previous = load_session(harness.session.id)?.compaction_commit;
        let publications = harness
            .chunks()
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
            .count();
        codex::mount(&harness.server, harness.session.id, false, true).await;
        let history = CodexHistory::default();
        let gate = codex::Gate::default();
        let agent = codex::agent(
            codex::model(&harness.server, history.clone())?,
            harness.session.id,
            gate.clone(),
        );
        let target = harness.session.id;
        let execute = Arc::new(move || {
            let agent = agent.clone();
            Box::pin(async move { codex::run(agent, target).await })
        });
        let recovery = execute.clone();
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
            target,
            Some(Arc::new(move |_, _| recovery())),
        );
        codelet_tools::set_generate_compaction_handler(target, Some(Arc::new(move |_| execute())));
        // @step When actual Codex exploration expires after its checkpoint model identity changes
        let call = async {
            if generated {
                use rig::tool::Tool;
                codelet_tools::GenerateCompactionTool::new(target)
                    .call(codelet_tools::GenerateCompactionArgs { session_id: None })
                    .await
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            } else {
                harness.call(false).await
            }
        };
        let corrupt = async {
            tokio::time::timeout(Duration::from_secs(1), gate.started.notified()).await?;
            let mut checkpoint = history.snapshot()?.ok_or("native checkpoint")?;
            checkpoint.model = Some("different-native-model".into());
            history.restore(Some(checkpoint))?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (result, boundary) = tokio::join!(call, corrupt);
        boundary?;
        gate.release.notify_one();
        tokio::time::timeout(Duration::from_secs(1), gate.finished.notified()).await?;
        // @step Then both routes report settlement failure without resetting the absolute deadline
        let error = result.expect_err("settlement must fail closed");
        assert!(
            error.contains("handoff settlement"),
            "generated={generated}: {error}"
        );
        assert_eq!(harness.bodies().await.len(), 1);
        // @step And the prior durable generation remains authoritative without a completion event
        assert_eq!(load_session(target)?.compaction_commit, previous);
        assert_eq!(
            harness
                .chunks()
                .iter()
                .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
                .count(),
            publications
        );
    }
    Ok(())
}
