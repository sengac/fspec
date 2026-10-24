//! Feature: spec/features/compaction-elapsed-time-runway.feature

#[path = "support/bug234_review.rs"]
mod review;
#[path = "support/bug234_scaffold.rs"]
mod scaffold;
#[path = "support/bug234_timing.rs"]
mod timing;

use codelet_cli::compaction_operation::{
    build_validated_dag, CompactionFailure, CompactionOperation,
};
use codelet_core::persistence::load_session;
use codelet_rpc_types::StreamChunk;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

#[tokio::test]
#[serial_test::serial]
async fn scenario_slow_exploration_yields_to_synthesis_on_both_compaction_routes(
) -> timing::TestResult {
    // @step Given both compaction routes gather original evidence within a finite operation budget
    for generated in [false, true] {
        let harness = timing::Harness::new().await?;
        let _budget = review::Budget::install(review::OPERATION_BUDGET);
        review::mount_stalled_model(&harness, false).await;
        // @step When exploration stalls after gathering evidence
        let result = tokio::time::timeout(Duration::from_secs(5), harness.call(generated)).await?;
        // @step Then tool-free synthesis runs before the operation deadline
        assert!(result.is_ok(), "elapsed runway must synthesize: {result:?}");
        let bodies = harness.bodies().await;
        let synthesis = bodies
            .iter()
            .find(|body| !timing::has_tools(body))
            .ok_or("synthesis")?;
        for original in ["committed evidence", "committed response"] {
            assert!(
                synthesis["messages"].to_string().contains(original),
                "route generated={generated}, missing original={original}"
            );
        }
        // @step And a valid complete summary is durably published
        assert!(load_session(harness.session.id)?
            .compaction_commit
            .is_some());
        assert!(harness
            .chunks()
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_stalled_synthesis_fails_closed_on_the_shared_deadline() -> timing::TestResult {
    // @step Given a compactor has an existing durable generation
    let harness = timing::Harness::new().await?;
    let _budget = review::Budget::install(review::OPERATION_BUDGET);
    harness.prior_generation().await?;
    let original = load_session(harness.session.id)?.compaction_commit;
    let publications = harness
        .chunks()
        .iter()
        .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
        .count();
    review::mount_stalled_model(&harness, true).await;
    // @step When synthesis cannot finish within its reserved time
    let failure = tokio::time::timeout(Duration::from_secs(5), harness.call(false))
        .await?
        .expect_err("stalled synthesis fails closed");
    // @step Then the operation reports a phase-specific deadline failure
    assert!(
        failure.contains("synthesis") && failure.contains("deadline"),
        "{failure}"
    );
    // @step And the durable generation remains unchanged without a completion event
    assert_eq!(
        load_session(harness.session.id)?.compaction_commit,
        original
    );
    assert!(harness
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    assert!(!harness
        .session
        .compaction_in_progress
        .load(Ordering::Acquire));
    assert_eq!(
        harness.session.get_status(),
        codelet_rpc_types::SessionStatus::Idle
    );
    assert_eq!(
        harness
            .chunks()
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
            .count(),
        publications
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn scenario_candidate_correction_inherits_the_remaining_operation_budget(
) -> timing::TestResult {
    // @step Given a compaction operation rejects its first candidate
    let mut operation = CompactionOperation::new(Duration::from_millis(120), 3);
    let identity = operation.begin()?;
    let attempts = Arc::new(AtomicUsize::new(0));
    let calls = attempts.clone();
    let deadlines = Arc::new(Mutex::new(Vec::new()));
    let observed = deadlines.clone();
    let started = tokio::time::Instant::now();
    // @step When bounded correction starts
    let result = operation
        .execute(async move {
            build_validated_dag("Build", &Default::default(), move |_| {
                observed.lock().expect("observed deadlines").push(
                    codelet_cli::compaction_timing::deadline()
                        .expect("inherited operation deadline"),
                );
                let attempt = calls.fetch_add(1, Ordering::AcqRel);
                async move {
                    if attempt == 0 {
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        Ok("invalid candidate".into())
                    } else {
                        std::future::pending::<Result<String, String>>().await
                    }
                }
            })
            .await
            .map_err(CompactionFailure::Builder)
        })
        .await;
    // @step Then the next attempt retains the original absolute deadline
    assert_eq!(attempts.load(Ordering::Acquire), 2);
    let captured = deadlines.lock().expect("captured deadlines");
    assert_eq!(captured[0], captured[1]);
    operation.resolve();
    assert_eq!(operation.begin()?, identity);
    // @step And stalled work cannot reset or outlive that deadline
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_millis(250));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_completed_evidence_survives_a_timeout_inside_an_unfinished_tool_batch(
) -> timing::TestResult {
    // @step Given streaming and nonstreaming compaction routes read original evidence before a slow command in the same response
    let mut failures = Vec::new();
    for (generated, streaming) in [(false, false), (true, false), (false, true)] {
        let result = async {
            let harness = timing::Harness::new().await?;
            let _budget = review::Budget::install(review::OPERATION_BUDGET);
            review::mount(&harness, true).await;
            if streaming {
                review::install_streaming(&harness);
            }
            // @step When exploration times out while the slow command is still unfinished
            let outcome =
                tokio::time::timeout(Duration::from_secs(5), harness.call(generated)).await?;
            let bodies = harness.bodies().await;
            outcome?;
            let synthesis = bodies
                .iter()
                .find(|body| !timing::has_tools(body))
                .ok_or("synthesis request")?;
            // @step Then synthesis retains the completed originals with their authentic assistant and tool pairing
            let messages = synthesis["messages"]
                .as_array()
                .or_else(|| synthesis["input"].as_array())
                .ok_or("synthesis messages")?;
            let result = messages
                .iter()
                .find(|message| {
                    (message["role"] == "tool" && message["tool_call_id"] == "originals-completed")
                        || (message["type"] == "function_call_output"
                            && message["call_id"] == "originals-completed")
                })
                .ok_or(format!(
                    "completed original result lost: generated={generated}, streaming={streaming}"
                ))?;
            for original in ["committed evidence", "committed response"] {
                assert!(
                    result["content"].to_string().contains(original)
                        || result["output"].to_string().contains(original),
                    "original evidence lost"
                );
            }
            let calls = messages
                .iter()
                .filter_map(|message| message["tool_calls"].as_array())
                .flatten()
                .chain(
                    messages
                        .iter()
                        .filter(|message| message["type"] == "function_call"),
                )
                .collect::<Vec<_>>();
            assert!(calls.iter().any(|call| call["id"] == "originals-completed"));
            // @step And accompanying assistant text and reasoning remain attached to the completed calls
            assert!(synthesis.to_string().contains(scaffold::ASSISTANT_TEXT));
            // @step And the unfinished command is not presented as completed
            assert!(!calls.iter().any(|call| call["id"] == "command-unfinished"));
            assert!(!messages
                .iter()
                .any(|message| message["tool_call_id"] == "command-unfinished"
                    || message["call_id"] == "command-unfinished"));
            // @step And only a validated complete summary is durably published
            assert_eq!(
                load_session(harness.session.id)?
                    .compaction
                    .ok_or("committed DAG")?
                    .summary,
                timing::DAG
            );
            assert!(harness
                .chunks()
                .iter()
                .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        if let Err(error) = result {
            failures.push(error.to_string());
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
    scaffold::verify_signed_reasoning().await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_synthesis_can_borrow_unused_time_beyond_a_candidate_estimate(
) -> timing::TestResult {
    // @step Given both compaction routes reserve synthesis time within one absolute operation deadline
    for generated in [false, true] {
        let harness = timing::Harness::new().await?;
        let _budget = review::Budget::install(review::OPERATION_BUDGET);
        review::mount(&harness, false).await;
        // @step When valid synthesis exceeds its candidate estimate but fits the remaining operation time
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(5), harness.call(generated)).await?;
        // @step Then synthesis completes without another exploration attempt or a reset deadline
        assert!(
            result.is_ok(),
            "unused operation time must remain usable: {result:?}"
        );
        assert!(started.elapsed() > review::OPERATION_BUDGET / 3);
        let bodies = harness.bodies().await;
        assert_eq!(
            bodies
                .iter()
                .filter(|body| !timing::has_tools(body))
                .count(),
            1
        );
        let exploration_count = bodies.iter().filter(|body| timing::has_tools(body)).count();
        assert!(
            (1..=2).contains(&exploration_count),
            "route generated={generated}, exploration_count={exploration_count}"
        );
        assert!(!timing::has_tools(
            bodies.last().ok_or("synthesis request")?
        ));
        for original in ["committed evidence", "committed response"] {
            assert!(bodies
                .last()
                .ok_or("synthesis request")?
                .to_string()
                .contains(original));
        }
        // @step And the validated summary is durably published before the operation expires
        assert!(started.elapsed() < review::OPERATION_BUDGET);
        assert!(load_session(harness.session.id)?
            .compaction_commit
            .is_some());
    }
    Ok(())
}
