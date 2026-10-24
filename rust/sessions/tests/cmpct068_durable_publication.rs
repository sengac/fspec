//! Feature: spec/features/durable-compaction-context-snapshot.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_sessions::committed_publication::context_snapshot;

#[test]
#[serial_test::serial]
fn scenario_runtime_snapshot_derives_occupancy_and_coverage_from_the_authoritative_durable_generation(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a durably promoted DAG with an estimated whole-request occupancy and verified suffix
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("runtime publication", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "original covered source")?;
    let mut candidate = boundary::candidate(&manifest)?;
    candidate.input_tokens = 120;
    append_message(&mut manifest, "user", "authoritative suffix")?;
    let committed = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    // @step When the runtime prepares its committed context event
    let snapshot = context_snapshot(&committed, 1000, 100.0, 200.0, false)?;
    // @step Then tokens coverage and generation match the authoritative durable boundary
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.tokens.input_tokens, 120);
    assert_eq!(
        snapshot.tokens.output_tokens, 0,
        "lifetime spend is not context occupancy"
    );
    assert_eq!(snapshot.covered_turns, Some(1));
    assert_eq!(snapshot.retained_turns, Some(1));
    assert!(snapshot.estimated);
    assert!(!snapshot.reconciled);
    // @step And trigger-relative fill remains distinct from absolute model capacity
    assert_eq!(
        snapshot.context_fill.fill_percentage, 120,
        "do not clamp trigger fill"
    );
    assert_eq!(snapshot.context_fill.threshold, 100.0);
    assert_eq!(snapshot.context_fill.context_window, 200.0);
    let acknowledged = publish_compaction(&committed, |_| Ok(()))?;
    let reconciled = context_snapshot(&acknowledged, 1000, 100.0, 200.0, true)?;
    assert!(reconciled.reconciled);
    // @step And stale or uncommitted manifests cannot produce a committed context event
    assert!(context_snapshot(&manifest, 1000, 100.0, 200.0, false).is_err());
    let mut stale = acknowledged.clone();
    stale
        .compaction_commit
        .as_mut()
        .ok_or("committed generation")?
        .identity
        .generation += 1;
    assert!(context_snapshot(&stale, 1000, 100.0, 200.0, false).is_err());
    for (trigger, capacity) in [
        (0.0, 200.0),
        (f64::NAN, 200.0),
        (100.0, f64::INFINITY),
        (201.0, 200.0),
    ] {
        assert!(context_snapshot(&acknowledged, 1000, trigger, capacity, false).is_err());
    }
    Ok(())
}
