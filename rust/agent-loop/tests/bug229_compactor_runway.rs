//! Feature: spec/features/compactor-occupancy-runway.feature

#[path = "support/bug210_completion_metrics.rs"]
mod metrics;
#[path = "support/bug229_runway.rs"]
mod runway;

use codelet_core::persistence::{get_session_messages_full, load_session};
use codelet_rpc_types::StreamChunk;

#[tokio::test]
#[serial_test::serial]
async fn scenario_both_compaction_routes_reserve_synthesis_runway_as_originals_grow(
) -> metrics::TestResult {
    // @step Given both compaction routes gather growing persisted original evidence with explicit limits
    for generated in [false, true] {
        let harness = runway::Harness::new(200_000, 10_000).await?;
        let original = get_session_messages_full(&load_session(harness.fixture.session.id)?)?;
        harness.mount(runway::DAG, false, false).await;
        // @step When another exploration round would consume the synthesis runway
        harness.call(generated).await?;
        // @step Then tool-free synthesis starts before depth exhaustion with every gathered original intact
        let bodies = harness.bodies().await;
        assert!(
            bodies.len() < 20,
            "occupancy must stop exploration before depth: {}",
            bodies.len()
        );
        assert!(bodies.len() > 4, "all four originals must be gathered");
        assert_eq!(
            bodies
                .iter()
                .filter(|body| !runway::has_tools(body))
                .count(),
            1
        );
        let synthesis = bodies.last().ok_or("synthesis")?;
        assert_eq!(synthesis["model"], "o3");
        assert_eq!(synthesis["max_completion_tokens"], runway::OUTPUT);
        for position in 0..4 {
            let sentinel = format!(
                "evidence-{position}-start {} evidence-{position}-end",
                "x".repeat(10_000)
            );
            assert!(synthesis["messages"].to_string().contains(&sentinel));
        }
        harness.assert_admitted().await?;
        // @step And the complete source-covering DAG passes durable commit and publication gates
        let retained = load_session(harness.fixture.session.id)?;
        assert_eq!(
            retained.compaction.as_ref().ok_or("compaction")?.summary,
            runway::DAG
        );
        assert_eq!(
            retained
                .compaction
                .as_ref()
                .ok_or("boundary")?
                .compacted_before_index,
            4
        );
        assert!(retained.compaction_commit.is_some());
        assert!(harness
            .fixture
            .chunks()
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
        assert_eq!(
            serde_json::to_value(get_session_messages_full(&retained)?)?,
            serde_json::to_value(original)?
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_evidence_that_cannot_fit_synthesis_preserves_the_prior_generation(
) -> metrics::TestResult {
    // @step Given both compaction routes have a prior durable generation and an unfit original evidence page
    for generated in [false, true] {
        let mut harness = runway::Harness::new(200_000, 55_000).await?;
        harness.prior_generation().await?;
        let original = load_session(harness.fixture.session.id)?;
        let originals = get_session_messages_full(&original)?;
        let publications = harness
            .fixture
            .chunks()
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
            .count();
        harness.limits.context_capacity = Some(60_000);
        harness.configure().await?;
        harness.mount(runway::DAG, false, false).await;
        // @step When gathering that page exhausts synthesis capacity
        let failure = harness
            .call(generated)
            .await
            .expect_err("unfit evidence fails closed");
        // @step Then compaction fails explicitly without dispatching oversized bytes or truncating evidence
        assert!(failure.contains("exceeds capacity"), "{failure}");
        let bodies = harness.bodies().await;
        assert_eq!(
            bodies.len(),
            1,
            "only the fitting initial request reaches transport"
        );
        assert!(bodies.iter().all(runway::has_tools));
        harness.assert_admitted().await?;
        // @step And the prior durable generation and all persisted originals remain unchanged
        let retained = load_session(harness.fixture.session.id)?;
        assert_eq!(retained.compaction_commit, original.compaction_commit);
        assert_eq!(
            serde_json::to_value(get_session_messages_full(&retained)?)?,
            serde_json::to_value(originals)?
        );
        assert_eq!(
            harness
                .fixture
                .chunks()
                .iter()
                .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
                .count(),
            publications
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_invalid_runway_synthesis_cannot_publish_a_candidate() -> metrics::TestResult {
    // @step Given both compaction routes reach occupancy synthesis with persisted original evidence
    for generated in [false, true] {
        let harness = runway::Harness::new(200_000, 10_000).await?;
        // @step When synthesis returns a DAG that omits the captured source boundary
        harness
            .mount(
                "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Missing\">Missing source</dag-node>",
                false,
                false,
            )
            .await;
        let failure = harness
            .call(generated)
            .await
            .expect_err("missing boundary rejected");
        // @step Then bounded correction fails without durable promotion or publication
        assert!(failure.contains("after 3 attempts"), "{failure}");
        let bodies = harness.bodies().await;
        assert_eq!(
            bodies
                .iter()
                .filter(|body| !runway::has_tools(body))
                .count(),
            3
        );
        assert!(
            bodies.len() < 40,
            "each correction must use occupancy runway"
        );
        harness.assert_admitted().await?;
        assert!(load_session(harness.fixture.session.id)?
            .compaction_commit
            .is_none());
        assert!(!harness
            .fixture
            .chunks()
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_during_runway_synthesis_cannot_publish_a_candidate(
) -> metrics::TestResult {
    // @step Given both compaction routes are waiting for occupancy synthesis
    for generated in [false, true] {
        let harness = runway::Harness::new(200_000, 10_000).await?;
        let original = get_session_messages_full(&load_session(harness.fixture.session.id)?)?;
        harness.mount(runway::DAG, true, false).await;
        // @step When the compaction operation is cancelled
        let failure = harness
            .call(generated)
            .await
            .expect_err("cancelled synthesis");
        assert!(failure.contains("cancel"), "{failure}");
        // @step Then no candidate is committed or published and persisted originals remain available
        let bodies = harness.bodies().await;
        assert!(
            bodies.len() < 20,
            "cancel at occupancy synthesis, not depth exhaustion"
        );
        assert_eq!(
            bodies
                .iter()
                .filter(|body| !runway::has_tools(body))
                .count(),
            1
        );
        harness.assert_admitted().await?;
        let retained = load_session(harness.fixture.session.id)?;
        assert!(retained.compaction_commit.is_none());
        assert_eq!(
            serde_json::to_value(get_session_messages_full(&retained)?)?,
            serde_json::to_value(original)?
        );
        assert!(!harness
            .fixture
            .chunks()
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_provider_errors_do_not_masquerade_as_capacity_exhaustion() -> metrics::TestResult
{
    // @step Given a compactor has gathered persisted original evidence
    let harness = runway::Harness::new(200_000, 10_000).await?;
    harness.mount(runway::DAG, false, true).await;
    // @step When the provider rejects exploration with an ordinary error mentioning capacity
    let failure = harness.call(false).await.expect_err("provider rejection");
    // @step Then the provider error is returned without tool-free synthesis
    assert!(
        failure.contains("ordinary provider capacity failure"),
        "{failure}"
    );
    let bodies = harness.bodies().await;
    assert_eq!(bodies.len(), 2);
    assert!(bodies.iter().all(runway::has_tools));
    Ok(())
}
