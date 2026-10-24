//! Feature: spec/features/committed-context-wire-snapshot.feature

#[test]
fn scenario_unknown_coverage_is_not_presented_as_measured_zero() -> Result<(), serde_json::Error> {
    // @step Given a committed context wire snapshot with unknown durable coverage
    let event = serde_json::json!({
        "CommittedCompaction": {
        "snapshot": {
            "generation": 7,
            "tokens": {"input_tokens": 200, "output_tokens": 0},
            "contextFill": {"fill_percentage": 20, "effective_tokens": 200.0,
                "threshold": 1000.0, "context_window": 2000.0},
            "originalTokens": 1050, "coveredTurns": null, "retainedTurns": null,
            "estimated": true, "reconciled": false
        }}
    });
    // @step When the snapshot is decoded and encoded again
    let chunk: codelet_rpc_types::StreamChunk = serde_json::from_value(event)?;
    let roundtrip = serde_json::to_value(chunk)?;
    // @step Then coverage remains unknown and generation and tokens remain unchanged
    assert_eq!(
        roundtrip["CommittedCompaction"]["snapshot"]["coveredTurns"],
        serde_json::Value::Null
    );
    assert_eq!(
        roundtrip["CommittedCompaction"]["snapshot"]["retainedTurns"],
        serde_json::Value::Null
    );
    assert_eq!(
        roundtrip["CommittedCompaction"]["snapshot"]["generation"],
        7
    );
    assert_eq!(
        roundtrip["CommittedCompaction"]["snapshot"]["tokens"]["input_tokens"],
        200
    );
    Ok(())
}

#[test]
fn scenario_reduction_uses_context_occupancy_independently_of_displayed_spend() {
    // @step Given a committed context snapshot with separate occupancy and cumulative spend
    let snapshot = codelet_rpc_types::CommittedContextSnapshot {
        generation: 1,
        tokens: codelet_rpc_types::TokenTracker {
            input_tokens: 333,
            output_tokens: 2900,
            reasoning_tokens: Some(1100),
            cumulative_billed_input: Some(5030),
            cumulative_billed_output: Some(2900),
            ..Default::default()
        },
        context_fill: codelet_rpc_types::ContextFillInfo {
            fill_percentage: 33,
            effective_tokens: 333.0,
            threshold: 1000.0,
            context_window: 2000.0,
        },
        original_tokens: 1000,
        covered_turns: Some(1),
        retained_turns: Some(1),
        estimated: true,
        reconciled: true,
    };
    // @step When its reduction is calculated
    let reduction = snapshot.reduction_percentage();
    // @step Then only its context occupancy is compared with the original context
    assert!(
        (reduction - 66.7).abs() < 0.001,
        "spend contaminated reduction: {reduction}"
    );
}
