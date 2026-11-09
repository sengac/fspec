//! Feature: spec/features/agent-manager-profile-diagnostics.feature
use super::*;

#[test]
fn scenario_attribute_runaway_subordinate_forwarding_recv_loop() {
    // @step Given the fspec binary is built with debug=1 and strip="none"
    //
    // The pure attribution walker is independent of the build profile — the
    // contract under test is "when a resolved stack arrives, it is attributed
    // correctly". We simulate a resolved stack directly.

    // @step Given a tokio worker thread is spinning inside spawn_subordinate_forwarding_task::recv_loop
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
                "codelet_agent_loop::agent_manager_handler::spawn_subordinate_forwarding_task::{{closure}}",
                "rust/agent-loop/src/agent_manager_handler.rs",
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

    // @step Then the top hot_stacks entry has a leaf frame whose symbol contains "spawn_subordinate_forwarding_task"
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

    // @step Then the leaf frame of that hot stack has a file path ending in "agent_manager_handler.rs"
    let leaf_file = top_stack.frames[0]
        .file
        .as_ref()
        .expect("leaf frame file path must be populated when DWARF info present");
    assert!(
        leaf_file.ends_with("agent_manager_handler.rs"),
        "leaf file must end with agent_manager_handler.rs, got {leaf_file:?}"
    );
}

#[test]
fn scenario_detect_stripped_build_and_report_debug_info_unavailable() {
    // @step Given a synthetic pprof report where fewer than 10 percent of samples have a non-noise frame with a resolvable source file
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
                "rust/tools/src/foo.rs",
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

    // @step Then the sampling section reports resolved_rust_samples less than total_samples divided by 10
    assert_eq!(output.sampling.total_samples, 100);
    assert!(
        output.sampling.resolved_rust_samples < output.sampling.total_samples / 10,
        "resolved_rust_samples must be < total_samples / 10, got {} vs {}",
        output.sampling.resolved_rust_samples,
        output.sampling.total_samples
    );

    // @step Then the sampling section includes a human-readable hint recommending a rebuild with debug info
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

#[test]
fn scenario_filter_samples_by_focus_substring() {
    // @step Given a synthetic pprof report containing stacks from multiple independent call chains
    let stacks = vec![
        SampleStack {
            thread_name: "tokio-runtime-worker-1".to_string(),
            thread_id: 11,
            frames: vec![rust_frame(
                "codelet_tools::bridge_relay::spawn_subordinate_forwarding_task",
                "rust/tools/src/bridge_relay.rs",
                100,
            )],
            count: 500,
        },
        SampleStack {
            thread_name: "tokio-runtime-worker-2".to_string(),
            thread_id: 12,
            frames: vec![rust_frame(
                "codelet_cli::interactive::stream_loop::run",
                "rust/cli/src/interactive/stream_loop.rs",
                200,
            )],
            count: 300,
        },
        SampleStack {
            thread_name: "main".to_string(),
            thread_id: 1,
            frames: vec![rust_frame(
                "codelet_core::session_manager::list_sessions",
                "rust/core/src/session_manager.rs",
                50,
            )],
            count: 100,
        },
    ];

    // @step When the profile result is built with focus set to a substring matching only one of those call chains
    let output = attribute_samples(
        &stacks,
        1.0,
        250,
        20,
        Some("spawn_subordinate_forwarding_task"),
    );

    // @step Then every entry in hot_stacks contains at least one frame whose symbol contains the focus substring
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

    // @step Then every entry in scopes_by_calls is attributed to a stack that contains the focus substring
    for scope in &output.scopes_by_calls {
        assert!(
            scope.label.contains("spawn_subordinate_forwarding_task"),
            "every scopes_by_calls entry must have been attributed to a focus-matching stack, \
             got {:?}",
            scope.label
        );
    }

    // @step Then samples_by_thread only reflects threads that ran stacks containing the focus substring
    for thread in &output.samples_by_thread {
        assert_eq!(
            thread.thread_name, "tokio-runtime-worker-1",
            "only tokio-runtime-worker-1 ran the focus-matching stack, got {:?}",
            thread.thread_name
        );
    }
}
