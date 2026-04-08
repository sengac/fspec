@done
@profiling
@agent-manager
@rust
@AMGR-018
Feature: Rewrite profile action to produce meaningful symbol resolution, per-thread attribution, and hot call stacks

  """
  Backward compatibility: existing scopes_by_calls and scopes_by_self_ms fields remain in ProfileResult with the same shape; new fields (samples_by_thread, hot_stacks, sampling) are additive so existing tool-call consumers do not break.
  Noise frame blocklist lives in session.rs: NOISE_FRAME_PREFIXES = ['libsystem_', '__pthread_', '_pthread_', '_dyld_', '_os_', '__os_unfair_', '_uv_', '_uv__', 'libuv', 'libgcc', 'libc', '_napi_register_module_v1', 'napi_', '__tsan', '__asan']. A symbol is considered noise if any prefix is a substring of its demangled name. The match uses str::contains for portability across macOS's underscore-prefixed mangling convention.
  hot_stacks uses a HashMap<Vec<String>, StackAggregate> keyed by the first 6 meaningful symbol names in each resolved stack. Each StackAggregate accumulates sample_count and tracks the originating thread_name. At build time the map is sorted by sample_count desc and truncated to top_n. File/line metadata is captured from the leaf meaningful frame only — full per-frame metadata is dropped to keep the payload under the 1MB tool response limit.
  cpu_cores_consumed is computed as total_samples * (1.0 / SAMPLE_FREQUENCY_HZ) / duration_secs. For a fully CPU-saturated single thread at 250Hz over 10s you'd expect ~2500 samples, giving cpu_cores_consumed ≈ 1.0. Two saturated workers would give ~2.0. This lets the agent immediately judge 'is the process actually hammering or is it mostly idle' without parsing individual sample counts.
  The resolved_rust_samples / unresolved_samples split uses a simple heuristic: a sample is 'resolved' if its attributed non-noise frame has file.is_some() AND filename doesn't end with '.c' or 'Unknown'. This gives a tight signal — if it's <10% the build is almost certainly stripped.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The release build must retain line-table debug info (debug=1) and NOT strip symbols, so backtrace-rs can resolve internal Rust functions
  #   2. run_pprof_window walks each stack leaf-to-root and attributes the sample to the first non-noise frame (skipping libsystem, libuv, pthread, dyld, _os_, __os_unfair, and NAPI C-export wrapper frames)
  #   3. When a physical stack frame contains multiple inlined symbols, only the outermost symbol is credited with the sample (no double-counting)
  #   4. ProfileResult includes samples_by_thread: a Vec<ThreadSampleReport> listing every thread that produced samples, sorted by sample_count desc, each with thread_name, thread_id, sample_count, and cpu_ms
  #   5. ProfileResult includes hot_stacks: a Vec<StackReport> listing the top-N unique call stacks by sample_count, each stack truncated to the first 6 meaningful frames with {symbol, file, line} and carrying thread_name
  #   6. ProfileResult includes sampling: a SamplingReport with total_samples, resolved_rust_samples, unresolved_samples, cpu_cores_consumed (derived from total samples and duration), and debug_info_available (false when < 10% of samples resolved to Rust symbols)
  #   7. A new focus: Option<String> argument drops any sample whose stack does not contain a frame whose symbol contains the focus substring — applied before per-leaf attribution so hot_stacks, scopes_by_calls and samples_by_thread all reflect the narrowed view
  #   8. The noise-frame blocklist is a compile-time static array in session.rs so the filter cost is one substring scan per frame; adding a new noise prefix is a single source edit
  #   9. The TypeScript tool binding in src/tools/agentManager.ts and its JSON schema accept the new focus parameter and document it in the action description
  #   10. Changing strip from 'symbols' to 'none' and adding debug=1 + split-debuginfo='packed' to [profile.release] retains line-table DWARF in a sibling .dSYM bundle on macOS without ballooning the .node binary size
  #
  # EXAMPLES:
  #   1. AI calls AgentManager profile duration_secs=10. The subordinate_forwarding task is in a tight recv loop on a tokio worker. samples_by_thread[0] = {thread_name:'tokio-runtime-worker-3', sample_count:2400, cpu_ms:9600}. hot_stacks[0].frames[0] = {symbol:'codelet_napi::agent_manager_handler::spawn_subordinate_forwarding_task::{{closure}}', file:'codelet/napi/src/agent_manager_handler.rs', line:243}. The agent immediately sees exactly which function is hot and on which worker.
  #   2. AI calls profile duration_secs=10 on a binary built with the old strip='symbols' profile. Response.sampling.debug_info_available=false, sampling.resolved_rust_samples=8, sampling.unresolved_samples=2076, and a human-readable hint is embedded. The AI reports back 'cannot diagnose — need rebuilt binary with debug info' instead of chasing phantom _napi_register_module_v1 hot spots.
  #   3. AI suspects the subordinate forwarding task and calls profile duration_secs=10 focus='spawn_subordinate_forwarding_task'. hot_stacks contains only stacks that touched that function; samples_by_thread shows which workers ran it; scopes_by_calls sums the attributed samples only within that call chain — letting the AI confirm or rule out the suspect in one call.
  #   4. AI profiles an idle process. samples_by_thread shows every worker with <5 samples, scopes_by_calls top entry is a tokio::runtime::park or epoll_wait, and sampling.cpu_cores_consumed is 0.02. The AI confirms there is no CPU hammering — the old bug would have reported _napi_register_module_v1 as the hot spot regardless.
  #   5. AI profiles a process hot on three different code paths. hot_stacks[0..3] show three distinct stacks each with different leaf symbols but NOT merged together, and scopes_by_calls attributes each via walking-leaf-to-root, giving three separate attribution entries instead of the old behavior of collapsing them all into a single shared wrapper frame.
  #   6. Inlined double-count fix verified: AI profiles a synthetic workload where compiler inlines 4 helpers into a single frame. Old code would credit 4 symbols per sample (4x inflation); new code credits the outermost symbol only, so scopes_by_calls total_self_ms matches real wall-clock CPU burn within sampling error.
  #
  # ========================================

  Background: User Story
    As a AI agent diagnosing a CPU hammering issue in fspec
    I want to invoke the AgentManager profile action and receive a useful Rust-level breakdown
    So that I can pinpoint which Rust function on which tokio worker thread is consuming CPU, without rebuilding in debug mode or attaching dtrace

  Scenario: Attribute runaway subordinate_forwarding recv loop to correct thread and function
    Given the fspec binary is built with debug=1 and strip="none"
    When an AI agent invokes AgentManager profile with duration_secs 10
    Then the response samples_by_thread list is sorted by sample_count descending
    Given a tokio worker thread is spinning inside spawn_subordinate_forwarding_task::recv_loop
    Then the top samples_by_thread entry identifies a tokio worker thread by name
    Then the top hot_stacks entry has a leaf frame whose symbol contains "spawn_subordinate_forwarding_task"
    Then the leaf frame of that hot stack has a file path ending in "agent_manager_handler.rs"


  Scenario: Detect stripped build and report debug info unavailable
    Given a synthetic pprof report where fewer than 10 percent of samples have a non-noise frame with a resolvable source file
    When the profile result is built from that report
    Then the sampling section has debug_info_available set to false
    Then the sampling section reports resolved_rust_samples less than total_samples divided by 10
    Then the sampling section includes a human-readable hint recommending a rebuild with debug info


  Scenario: Filter samples by focus substring to narrow to a single call chain
    Given a synthetic pprof report containing stacks from multiple independent call chains
    When the profile result is built with focus set to a substring matching only one of those call chains
    Then every entry in hot_stacks contains at least one frame whose symbol contains the focus substring
    Then every entry in scopes_by_calls is attributed to a stack that contains the focus substring
    Then samples_by_thread only reflects threads that ran stacks containing the focus substring


  Scenario: Walk leaf to root skipping noise frames for attribution
    Given a synthetic stack whose leaf is __os_unfair_lock_lock_slow followed by _napi_register_module_v1 followed by a Rust function spawn_subordinate_forwarding_task
    When the profile session attributes the sample
    Then the attributed label is spawn_subordinate_forwarding_task and not __os_unfair_lock_lock_slow
    Then the attributed label is not _napi_register_module_v1


  Scenario: Credit only the outermost inlined symbol per physical frame
    Given a synthetic stack whose leaf frame contains four inlined symbols representing one call site
    When the profile session attributes the sample
    Then only one scopes_by_calls entry is incremented and the sample count equals 1
    Then the attributed label matches the outermost inlined symbol name


  Scenario: Report cpu_cores_consumed from total sample count
    Given a synthetic pprof report with 2500 samples captured at SAMPLE_FREQUENCY_HZ 250 over a 10 second window
    When the profile result is built
    Then sampling.cpu_cores_consumed is approximately 1.0 within sampling tolerance


  Scenario: Preserve backward compatibility of existing ProfileResult fields
    Given any ProfileResult serialized to JSON
    When the JSON is inspected
    Then the top-level object still contains duration_secs started_at ended_at process runtime scopes_by_calls scopes_by_self_ms and channels fields
    Then the top-level object also contains the new samples_by_thread hot_stacks and sampling fields

