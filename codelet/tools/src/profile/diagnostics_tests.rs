//! AMGR-018 — AgentManager profile diagnostics rewrite tests
//!
//! Feature: spec/features/agent-manager-profile-diagnostics.feature
//!
//! These tests validate the acceptance criteria for the profile-action rewrite:
//! noise-frame attribution, per-thread sample buckets, hot-stack aggregation,
//! sampling-quality reports, focus filtering, and backward compatibility of
//! `ProfileResult`. All tests are expected to FAIL during the red phase
//! because `attribute_samples` is stubbed with `todo!()`.
//!
//! Test runner: `cargo test -p codelet-tools profile::diagnostics_tests`

use super::attribution::{
    attribute_samples, AttributionOutput, FrameInfo, SampleStack, NOISE_FRAME_PREFIXES,
};
use super::result::{ProfileResult, SamplingReport};

// --------------------------------------------------------------------------
// Shared synthetic-sample helpers
// --------------------------------------------------------------------------

/// Build a single-symbol resolved physical frame with file+line.
fn rust_frame(symbol: &str, file: &str, line: u32) -> Vec<FrameInfo> {
    vec![FrameInfo {
        symbol: symbol.to_string(),
        file: Some(file.to_string()),
        line: Some(line),
    }]
}

/// Build a single-symbol unresolved physical frame (no file / line).
fn noise_frame(symbol: &str) -> Vec<FrameInfo> {
    vec![FrameInfo {
        symbol: symbol.to_string(),
        file: None,
        line: None,
    }]
}

/// Build a physical frame with four inlined symbols (leaf-most → leaf).
fn inlined_frame(outermost: &str, inner_a: &str, inner_b: &str, inner_c: &str) -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            symbol: outermost.to_string(),
            file: Some("codelet/tools/src/bridge_relay.rs".to_string()),
            line: Some(42),
        },
        FrameInfo {
            symbol: inner_a.to_string(),
            file: Some("codelet/tools/src/bridge_relay.rs".to_string()),
            line: Some(43),
        },
        FrameInfo {
            symbol: inner_b.to_string(),
            file: Some("codelet/tools/src/bridge_relay.rs".to_string()),
            line: Some(44),
        },
        FrameInfo {
            symbol: inner_c.to_string(),
            file: Some("codelet/tools/src/bridge_relay.rs".to_string()),
            line: Some(45),
        },
    ]
}

/// Helper: construct an AMGR-017-shaped `ProfileResult` stub with default
/// AMGR-018 fields to exercise the serde round-trip in scenario 7.
fn empty_profile_result() -> ProfileResult {
    ProfileResult {
        duration_secs: 1,
        started_at: "2026-04-08T00:00:00Z".to_string(),
        ended_at: "2026-04-08T00:00:01Z".to_string(),
        process: super::result::ProcessReport {
            pid: 1,
            rss_bytes_start: 1,
            rss_bytes_end: 1,
            total_threads_start: 1,
            total_threads_end: 1,
        },
        runtime: super::result::RuntimeReport {
            worker_threads: 1,
            alive_tokio_tasks_start: None,
            alive_tokio_tasks_end: None,
        },
        scopes_by_calls: Vec::new(),
        scopes_by_self_ms: Vec::new(),
        channels: Vec::new(),
        samples_by_thread: Vec::new(),
        hot_stacks: Vec::new(),
        sampling: SamplingReport::default(),
    }
}

// ==========================================================================
// Scenario 1: Attribute runaway subordinate_forwarding recv loop to correct
//             thread and function
// ==========================================================================

#[test]
fn scenario_attribute_runaway_subordinate_forwarding_recv_loop() {
    // @step Given the fspec binary is built with debug=1 and strip="none"
    //
    // The pure attribution walker is independent of the build profile — the
    // contract under test is "when a resolved stack arrives, it is attributed
    // correctly". We simulate a resolved stack directly.

    // @step Given a tokio worker thread is spinning inside
    //        spawn_subordinate_forwarding_task::recv_loop
    //
    // Build 2400 samples worth of stacks all on the same tokio worker hitting
    // a noise-leaf followed by the real Rust function.
    let stack = SampleStack {
        thread_name: "tokio-runtime-worker-3".to_string(),
        thread_id: 17,
        frames: vec![
            // Leaf: noise
            noise_frame("__psynch_mutexwait"),
            // Then: the real Rust function we want to be attributed to
            rust_frame(
                "codelet_napi::agent_manager_handler::spawn_subordinate_forwarding_task::{{closure}}",
                "codelet/napi/src/agent_manager_handler.rs",
                243,
            ),
            rust_frame(
                "tokio::runtime::task::core::Core::poll",
                "tokio/src/runtime/task/core.rs",
                300,
            ),
        ],
        count: 2400,
    };

    // @step When an AI agent invokes AgentManager profile with duration_secs 10
    let output: AttributionOutput =
        attribute_samples(std::slice::from_ref(&stack), 10.0, 250, 20, None);

    // @step Then the response samples_by_thread list is sorted by sample_count descending
    for pair in output.samples_by_thread.windows(2) {
        assert!(
            pair[0].sample_count >= pair[1].sample_count,
            "samples_by_thread must be sorted by sample_count desc"
        );
    }

    // @step Then the top samples_by_thread entry identifies a tokio worker thread by name
    let top_thread = output
        .samples_by_thread
        .first()
        .expect("samples_by_thread must include at least one entry");
    assert!(
        top_thread.thread_name.contains("tokio-runtime-worker"),
        "top thread must identify a tokio worker, got {:?}",
        top_thread.thread_name
    );
    assert_eq!(top_thread.sample_count, 2400);

    // @step Then the top hot_stacks entry has a leaf frame whose symbol contains
    //        "spawn_subordinate_forwarding_task"
    let top_stack = output
        .hot_stacks
        .first()
        .expect("hot_stacks must include at least one entry");
    let leaf_symbol = &top_stack
        .frames
        .first()
        .expect("hot_stacks[0] must have a leaf frame")
        .symbol;
    assert!(
        leaf_symbol.contains("spawn_subordinate_forwarding_task"),
        "leaf frame must be spawn_subordinate_forwarding_task, got {leaf_symbol:?}"
    );

    // @step Then the leaf frame of that hot stack has a file path ending in
    //        "agent_manager_handler.rs"
    let leaf_file = top_stack.frames[0]
        .file
        .as_ref()
        .expect("leaf frame file path must be populated when DWARF info present");
    assert!(
        leaf_file.ends_with("agent_manager_handler.rs"),
        "leaf file must end with agent_manager_handler.rs, got {leaf_file:?}"
    );
}

// ==========================================================================
// Scenario 2: Detect stripped build and report debug info unavailable
// ==========================================================================

#[test]
fn scenario_detect_stripped_build_and_report_debug_info_unavailable() {
    // @step Given a synthetic pprof report where fewer than 10 percent of
    //        samples have a non-noise frame with a resolvable source file
    //
    // 100 samples total; only 5 are resolved Rust frames; 95 are attributed
    // to an unresolved NAPI C wrapper (no file, no line).
    let mut stacks = Vec::new();
    for _ in 0..5 {
        stacks.push(SampleStack {
            thread_name: "main".to_string(),
            thread_id: 1,
            frames: vec![rust_frame(
                "codelet_tools::foo::bar",
                "codelet/tools/src/foo.rs",
                10,
            )],
            count: 1,
        });
    }
    for _ in 0..95 {
        stacks.push(SampleStack {
            thread_name: "main".to_string(),
            thread_id: 1,
            frames: vec![FrameInfo {
                symbol: "_napi_register_module_v1".to_string(),
                file: None,
                line: None,
            }]
            .into_iter()
            .map(|f| vec![f])
            .collect(),
            count: 1,
        });
    }

    // @step When the profile result is built from that report
    let output = attribute_samples(&stacks, 1.0, 250, 20, None);

    // @step Then the sampling section has debug_info_available set to false
    assert!(
        !output.sampling.debug_info_available,
        "debug_info_available must be false when < 10% of samples resolve"
    );

    // @step Then the sampling section reports resolved_rust_samples less than
    //        total_samples divided by 10
    assert_eq!(output.sampling.total_samples, 100);
    assert!(
        output.sampling.resolved_rust_samples < output.sampling.total_samples / 10,
        "resolved_rust_samples must be < total_samples / 10, got {} vs {}",
        output.sampling.resolved_rust_samples,
        output.sampling.total_samples
    );

    // @step Then the sampling section includes a human-readable hint
    //        recommending a rebuild with debug info
    let hint = output
        .sampling
        .hint
        .as_ref()
        .expect("hint must be populated when debug info is missing");
    assert!(
        hint.to_lowercase().contains("rebuild") || hint.to_lowercase().contains("debug"),
        "hint must recommend a rebuild/debug step, got {hint:?}"
    );
}

// ==========================================================================
// Scenario 3: Filter samples by focus substring to narrow to a single call chain
// ==========================================================================

#[test]
fn scenario_filter_samples_by_focus_substring() {
    // @step Given a synthetic pprof report containing stacks from multiple
    //        independent call chains
    let stacks = vec![
        SampleStack {
            thread_name: "tokio-runtime-worker-1".to_string(),
            thread_id: 11,
            frames: vec![rust_frame(
                "codelet_tools::bridge_relay::spawn_subordinate_forwarding_task",
                "codelet/tools/src/bridge_relay.rs",
                100,
            )],
            count: 500,
        },
        SampleStack {
            thread_name: "tokio-runtime-worker-2".to_string(),
            thread_id: 12,
            frames: vec![rust_frame(
                "codelet_cli::interactive::stream_loop::run",
                "codelet/cli/src/interactive/stream_loop.rs",
                200,
            )],
            count: 300,
        },
        SampleStack {
            thread_name: "main".to_string(),
            thread_id: 1,
            frames: vec![rust_frame(
                "codelet_core::session_manager::list_sessions",
                "codelet/core/src/session_manager.rs",
                50,
            )],
            count: 100,
        },
    ];

    // @step When the profile result is built with focus set to a substring
    //        matching only one of those call chains
    let output = attribute_samples(
        &stacks,
        1.0,
        250,
        20,
        Some("spawn_subordinate_forwarding_task"),
    );

    // @step Then every entry in hot_stacks contains at least one frame whose
    //        symbol contains the focus substring
    assert!(
        !output.hot_stacks.is_empty(),
        "hot_stacks must be non-empty after focus filter"
    );
    for stack in &output.hot_stacks {
        let has_focus = stack
            .frames
            .iter()
            .any(|f| f.symbol.contains("spawn_subordinate_forwarding_task"));
        assert!(
            has_focus,
            "every hot_stacks entry must contain focus substring, got {:?}",
            stack.frames
        );
    }

    // @step Then every entry in scopes_by_calls is attributed to a stack that
    //        contains the focus substring
    for scope in &output.scopes_by_calls {
        assert!(
            scope.label.contains("spawn_subordinate_forwarding_task"),
            "every scopes_by_calls entry must have been attributed to a focus-matching stack, \
             got {:?}",
            scope.label
        );
    }

    // @step Then samples_by_thread only reflects threads that ran stacks
    //        containing the focus substring
    for thread in &output.samples_by_thread {
        assert_eq!(
            thread.thread_name, "tokio-runtime-worker-1",
            "only tokio-runtime-worker-1 ran the focus-matching stack, got {:?}",
            thread.thread_name
        );
    }
}

// ==========================================================================
// Scenario 4: Walk leaf to root skipping noise frames for attribution
// ==========================================================================

#[test]
fn scenario_walk_leaf_to_root_skipping_noise_frames() {
    // @step Given a synthetic stack whose leaf is __os_unfair_lock_lock_slow
    //        followed by _napi_register_module_v1 followed by a Rust function
    //        spawn_subordinate_forwarding_task
    let stack = SampleStack {
        thread_name: "tokio-runtime-worker".to_string(),
        thread_id: 5,
        frames: vec![
            noise_frame("__os_unfair_lock_lock_slow"),
            noise_frame("_napi_register_module_v1"),
            rust_frame(
                "codelet_tools::bridge_relay::spawn_subordinate_forwarding_task",
                "codelet/tools/src/bridge_relay.rs",
                321,
            ),
        ],
        count: 42,
    };

    // @step When the profile session attributes the sample
    let output = attribute_samples(std::slice::from_ref(&stack), 1.0, 250, 20, None);

    // @step Then the attributed label is spawn_subordinate_forwarding_task
    //        and not __os_unfair_lock_lock_slow
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

// ==========================================================================
// Scenario 5: Credit only the outermost inlined symbol per physical frame
// ==========================================================================

#[test]
fn scenario_credit_only_outermost_inlined_symbol_per_physical_frame() {
    // @step Given a synthetic stack whose leaf frame contains four inlined
    //        symbols representing one call site
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

    // @step Then only one scopes_by_calls entry is incremented and the sample
    //        count equals 1
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

// ==========================================================================
// Scenario 6: Report cpu_cores_consumed from total sample count
// ==========================================================================

#[test]
fn scenario_report_cpu_cores_consumed_from_total_sample_count() {
    // @step Given a synthetic pprof report with 2500 samples captured at
    //        SAMPLE_FREQUENCY_HZ 250 over a 10 second window
    let stack = SampleStack {
        thread_name: "tokio-runtime-worker".to_string(),
        thread_id: 7,
        frames: vec![rust_frame(
            "codelet_tools::hot::burn",
            "codelet/tools/src/hot.rs",
            1,
        )],
        count: 2500,
    };

    // @step When the profile result is built
    let output = attribute_samples(std::slice::from_ref(&stack), 10.0, 250, 20, None);

    // @step Then sampling.cpu_cores_consumed is approximately 1.0 within
    //        sampling tolerance
    let cores = output.sampling.cpu_cores_consumed;
    assert!(
        (cores - 1.0).abs() < 0.05,
        "cpu_cores_consumed must be ~1.0 for 2500 samples at 250 Hz over 10 s, got {cores}"
    );
    assert_eq!(output.sampling.total_samples, 2500);
}

// ==========================================================================
// Scenario 7: Preserve backward compatibility of existing ProfileResult fields
// ==========================================================================

#[test]
fn scenario_preserve_backward_compatibility_of_existing_profile_result_fields() {
    // @step Given any ProfileResult serialized to JSON
    let profile = empty_profile_result();

    // @step When the JSON is inspected
    let json = serde_json::to_value(&profile).expect("ProfileResult must serialise to JSON");

    // @step Then the top-level object still contains duration_secs started_at
    //        ended_at process runtime scopes_by_calls scopes_by_self_ms and
    //        channels fields
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

    // @step Then the top-level object also contains the new samples_by_thread
    //        hot_stacks and sampling fields
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
