//! Feature: spec/features/agent-manager-profile-diagnostics.feature
use super::*;

#[test]
fn scenario_walk_leaf_to_root_skipping_noise_frames() {
    // @step Given a synthetic stack whose leaf is __os_unfair_lock_lock_slow followed by _napi_register_module_v1 followed by a Rust function spawn_subordinate_forwarding_task
    let stack = SampleStack {
        thread_name: "tokio-runtime-worker".to_string(),
        thread_id: 5,
        frames: vec![
            noise_frame("__os_unfair_lock_lock_slow"),
            noise_frame("_napi_register_module_v1"),
            rust_frame(
                "codelet_tools::bridge_relay::spawn_subordinate_forwarding_task",
                "rust/tools/src/bridge_relay.rs",
                321,
            ),
        ],
        count: 42,
    };

    // @step When the profile session attributes the sample
    let output = attribute_samples(std::slice::from_ref(&stack), 1.0, 250, 20, None);

    // @step Then the attributed label is spawn_subordinate_forwarding_task and not __os_unfair_lock_lock_slow
    let top = output
        .scopes_by_calls
        .first()
        .expect("at least one scope must be attributed after walking past noise");
    assert!(
        top.label.contains("spawn_subordinate_forwarding_task"),
        "top scope must be spawn_subordinate_forwarding_task, got {:?}",
        top.label
    );
    assert!(
        !top.label.contains("__os_unfair_lock_lock_slow"),
        "noise leaf must be skipped, got {:?}",
        top.label
    );

    // @step Then the attributed label is not _napi_register_module_v1
    assert!(
        !top.label.contains("_napi_register_module_v1"),
        "NAPI wrapper must be skipped, got {:?}",
        top.label
    );

    // Sanity: both blocklist prefixes are still in the noise set so this
    // contract cannot drift.
    assert!(NOISE_FRAME_PREFIXES.iter().any(|p| p.contains("os_unfair")));
    assert!(NOISE_FRAME_PREFIXES
        .iter()
        .any(|p| p.contains("napi_register_module_v1")));
}

#[test]
fn scenario_credit_only_outermost_inlined_symbol_per_physical_frame() {
    // @step Given a synthetic stack whose leaf frame contains four inlined symbols representing one call site
    let stack = SampleStack {
        thread_name: "main".to_string(),
        thread_id: 1,
        frames: vec![inlined_frame(
            "codelet_tools::bridge_relay::outer_wrapper",
            "codelet_tools::bridge_relay::middle_helper",
            "codelet_tools::bridge_relay::inner_helper",
            "codelet_tools::bridge_relay::leaf_helper",
        )],
        count: 1,
    };

    // @step When the profile session attributes the sample
    let output = attribute_samples(std::slice::from_ref(&stack), 1.0, 250, 20, None);

    // @step Then only one scopes_by_calls entry is incremented and the sample count equals 1
    assert_eq!(
        output.scopes_by_calls.len(),
        1,
        "exactly one scope must be credited (no inlined double-counting), got {}",
        output.scopes_by_calls.len()
    );
    assert_eq!(
        output.scopes_by_calls[0].call_count, 1,
        "the one credited scope must have call_count=1"
    );

    // @step Then the attributed label matches the outermost inlined symbol name
    assert!(
        output.scopes_by_calls[0].label.contains("outer_wrapper"),
        "attributed label must be the outermost inlined symbol, got {:?}",
        output.scopes_by_calls[0].label
    );
    assert!(
        !output.scopes_by_calls[0].label.contains("leaf_helper"),
        "the innermost inlined symbol must NOT be credited, got {:?}",
        output.scopes_by_calls[0].label
    );
}

#[test]
fn scenario_report_cpu_cores_consumed_from_total_sample_count() {
    // @step Given a synthetic pprof report with 2500 samples captured at SAMPLE_FREQUENCY_HZ 250 over a 10 second window
    let stack = SampleStack {
        thread_name: "tokio-runtime-worker".to_string(),
        thread_id: 7,
        frames: vec![rust_frame(
            "codelet_tools::hot::burn",
            "rust/tools/src/hot.rs",
            1,
        )],
        count: 2500,
    };

    // @step When the profile result is built
    let output = attribute_samples(std::slice::from_ref(&stack), 10.0, 250, 20, None);

    // @step Then sampling.cpu_cores_consumed is approximately 1.0 within sampling tolerance
    let cores = output.sampling.cpu_cores_consumed;
    assert!(
        (cores - 1.0).abs() < 0.05,
        "cpu_cores_consumed must be ~1.0 for 2500 samples at 250 Hz over 10 s, got {cores}"
    );
    assert_eq!(output.sampling.total_samples, 2500);
}

#[test]
fn scenario_preserve_backward_compatibility_of_existing_profile_result_fields() {
    // @step Given any ProfileResult serialized to JSON
    let profile = empty_profile_result();

    // @step When the JSON is inspected
    let json = serde_json::to_value(&profile).expect("ProfileResult must serialise to JSON");

    // @step Then the top-level object still contains duration_secs started_at ended_at process runtime scopes_by_calls scopes_by_self_ms and channels fields
    for required in [
        "duration_secs",
        "started_at",
        "ended_at",
        "process",
        "runtime",
        "scopes_by_calls",
        "scopes_by_self_ms",
        "channels",
    ] {
        assert!(
            json.get(required).is_some(),
            "existing field {required} must still be present in the JSON output"
        );
    }

    // @step Then the top-level object also contains the new samples_by_thread hot_stacks and sampling fields
    for required in ["samples_by_thread", "hot_stacks", "sampling"] {
        assert!(
            json.get(required).is_some(),
            "new AMGR-018 field {required} must be present in the JSON output"
        );
    }

    // Round-trip: deserialising the same JSON into ProfileResult must succeed
    // (serde default on the new fields keeps old tool-call consumers working).
    let round_trip: ProfileResult =
        serde_json::from_value(json).expect("ProfileResult must round-trip through JSON");
    assert_eq!(round_trip, profile);
}
