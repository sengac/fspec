@done
@rig
@performance
@debugging
@napi
@rust
@tools
@profiling
@agent-manager
@AMGR-017
Feature: Add profile action to AgentManager for Rust runtime diagnostics
  """
  Instrumentation API: profile_scope!("label") macro is the primary call site marker. Compiles to `let _guard = ProfileScope::enter(concat!(module_path!(), "::", "label"));` which on Drop accumulates elapsed nanos into ScopeMetrics. The label MUST be a &'static str (compile-time concat) so the registry never allocates on the hot path
  Initial instrumentation targets (must be wired during implementing — not optional): codelet/tools/src/bridge_relay.rs relay_loop outer loop, connect_and_relay inbound for-loop, connect_and_relay outbound select! arms (one label per arm: control_recv, stream_recv, subordinate_recv, shutdown_recv); codelet/napi/src/agent_manager_handler.rs handle_await_idle outer loop and join_set.join_next branches, spawn_subordinate_forwarding_task recv loop; codelet/napi/src/scheduler/engine.rs spawn_scheduler tick loop; codelet/napi/src/scheduler/loop_store.rs register_with_task_and_idle_check inner loop
  Channel instrumentation: introduce TrackedBroadcast<T>, TrackedMpsc<T>, and TrackedUnboundedMpsc<T> wrapper types in codelet/tools/src/profile/channels.rs that delegate to the tokio primitives but register themselves in a global ChannelRegistry on construction (with a stable name) and unregister on Drop. Migrate supervisor_broadcast in codelet/napi/src/session_manager.rs:509, OUTBOUND_CONTROL_SENDERS in bridge_relay.rs, and SUBORDINATE_CHUNK_SENDERS in bridge_relay.rs to use these wrappers
  Tokio runtime metrics: use tokio's experimental tokio::runtime::Handle::current().metrics() (requires `tokio_unstable` cfg flag in build.rs) to populate the runtime section. If tokio_unstable is not enabled, the runtime section degrades gracefully to {worker_threads: <known>, alive_tokio_tasks: null}
  Process metrics: use the existing dependency tree where possible — RSS via mach2::task on macOS, /proc/self/status on Linux, GetProcessMemoryInfo on Windows. Wrap in a single get_process_metrics() helper in codelet/common/src/process_metrics.rs to avoid duplicating platform code
  AgentManager dispatch: extend AgentManagerAction enum in codelet/tools/src/agent_manager/types.rs with `Profile { duration_secs: Option<u32>, top_n: Option<usize>, label_prefix: Option<String> }`. Profile is ASYNC (it awaits tokio::time::sleep for the duration of the window), so it dispatches through the existing async handler in codelet/napi/src/agent_manager_handler.rs::create_async_handler — NOT through the sync handler. The handler validates duration_secs is in range 1..=60 (defaulting to 10), then awaits ProfileSession::run(duration_secs, top_n, label_prefix)
  Gating mechanism: ProfileRegistry exposes `static PROFILING_ACTIVE: AtomicBool = AtomicBool::new(false)`. The profile_scope!() macro expands to `let _guard = if PROFILING_ACTIVE.load(Ordering::Relaxed) { Some(ProfileScope::enter(concat!(module_path!(), "::", $label))) } else { None };` — when inactive this is one Relaxed atomic load + branch (sub-1ns on aarch64). ProfileSession::run() does: (1) compare_exchange(false, true, AcqRel, Acquire) — return error if Err, (2) call registry.reset_all() to zero counters and capture baseline metrics, (3) tokio::time::sleep(Duration::from_secs(duration_secs)).await, (4) capture end metrics and build result, (5) store(false, Release) on PROFILING_ACTIVE, (6) return result. The Acquire/Release pairing ensures all counter writes during the window are visible to the result-building code
  Module path: codelet/tools/src/profile/ — new submodule containing registry.rs (ProfileRegistry singleton with DashMap<&'static str, ScopeMetrics> and PROFILING_ACTIVE: AtomicBool), scope.rs (ProfileScope RAII guard + profile_scope!() macro), session.rs (ProfileSession::run() — the time-bounded capture orchestrator), result.rs (ProfileResult, ScopeReport, ChannelReport types), and channels.rs (instrumented broadcast/mpsc wrappers used by bridge_relay and session_manager)
  rig::tool::Tool integration: extend AgentManagerTool::call() in codelet/tools/src/agent_manager/handler.rs to handle AgentManagerAction::Profile by awaiting ProfileSession::run(duration_secs, top_n, label_prefix). The handler must use the same singleton ProfileRegistry instance as the NAPI dispatch path so both call paths see the same counters and the same PROFILING_ACTIVE gate
  TypeScript binding: AgentManager tool wrapper in src/tools/agentManager.ts must add `profile` to the action union and forward duration_secs, top_n, and label_prefix parameters to the NAPI handler. Add the profile action to the JSON schema in src/tools/agentManagerSchema.ts so AI agents see it in their tool catalog. Document in the schema that the call BLOCKS for duration_secs (default 10) — agents must expect a long-running tool call and not interpret the wait as a hang
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentManager exposes a new `profile` action alongside spawn/list/get_status/close/message/set_role/await_idle, dispatched through both the NAPI handler path and the rig::tool::Tool trait so any caller (TUI, subordinate, supervisor, MCP, LLM tool_use) can invoke it
  #   2. Functions and async loop bodies are instrumented with a `#[profile_loop]` proc-macro (or manual `ProfileScope::enter("label")` RAII guard) that increments a per-label call counter, accumulates wall-clock self time in nanoseconds, and tracks max iteration time — stored in a global ProfileRegistry
  #   3. Every long-running async loop in codelet/tools/src/bridge_relay.rs (relay_loop, inbound_handle for-loop, outbound select! loop, subordinate forwarding loop), codelet/napi/src/agent_manager_handler.rs (await_idle outer loop, spawn_subordinate_forwarding_task receive loop), codelet/napi/src/scheduler/engine.rs (30s tick loop), and codelet/napi/src/scheduler/loop_store.rs (per-entry interval loop) MUST be instrumented with a unique label so each loop iteration is counted individually
  #   4. ProfileRegistry stores per-label metrics: { call_count: AtomicU64, total_self_ns: AtomicU64, max_iter_ns: AtomicU64, last_seen_unix_ms: AtomicU64, currently_executing: AtomicI32 } — all updates use Relaxed atomics so instrumentation overhead is < 100ns per call
  #   5. Profiling can be disabled at compile time via a `profile` Cargo feature flag (default on for dev/release, off for tiny builds) — when off, the macro/RAII guards expand to no-ops and the registry is omitted, so there is zero overhead
  #   6. The profile action takes a `duration_secs` integer parameter (range 1-60, default 10) specifying the length of the profiling window in seconds — instrumentation is active only during this window and returns aggregated results when it closes, mirroring how `sample(1)` and `perf record` operate as bounded captures rather than always-on systems
  #   7. Instrumentation is gated by a single global PROFILING_ACTIVE: AtomicBool flag in the ProfileRegistry — when false (the steady state), the profile_scope!() macro performs only one Relaxed atomic load and a branch-not-taken (~1ns), so leaving instrumentation compiled in costs effectively zero. Counter increments and timing only occur when the flag is true
  #   8. Only one profile session may run at a time. Starting a profile uses an atomic compare-and-swap on PROFILING_ACTIVE from false→true; if a session is already active, the second call returns an error of the form `{ error: "profile_session_active", started_at: <iso8601>, ends_in_secs: <n> }` rather than waiting in line or stomping the existing session
  #   9. Profile session lifecycle: (1) compare-and-swap PROFILING_ACTIVE to true, (2) reset all per-scope counters to zero and capture process/runtime baseline metrics, (3) tokio::time::sleep(duration_secs), (4) capture process/runtime end metrics and channel stats, (5) set PROFILING_ACTIVE to false, (6) return aggregated result. The profile call BLOCKS the calling agent for the full duration — this is intentional and the caller should expect it
  #   10. Profile result shape: { duration_secs, started_at, ended_at, process: { pid, rss_bytes_start, rss_bytes_end, total_threads_start, total_threads_end }, runtime: { worker_threads, alive_tokio_tasks_start, alive_tokio_tasks_end }, scopes_by_calls: [top N scopes sorted by call_count desc], scopes_by_self_ms: [top N scopes sorted by total_self_ms desc], channels: [{ name, sender_count, receiver_count, queued_at_end, lagged_during_window }] }. Each scope entry includes { label, call_count, total_self_ms, max_iter_ms, calls_per_sec, currently_executing_at_end }
  #   11. Optional `top_n` parameter (default 20, max 200) caps how many scopes appear in scopes_by_calls and scopes_by_self_ms. Optional `label_prefix` parameter filters scopes to those whose label starts with the given string before sorting/truncation — both filters apply at result-build time, never on the hot path
  #   12. The profile action is read-only with respect to session state — it must not spawn new tokio tasks during the window (sleep + atomic counters only), must not mutate any AgentManager session, and must not allocate on the hot path. Total instrumentation overhead during an active window must remain under 5% of CPU even at multi-million-calls-per-second hot loops
  #   13. No persistence — profile results live only in the tool-call response. There is no on-disk snapshot store, no LRU cache of past sessions, no ~/.fspec/profile-snapshots/ directory. Each profile call is a fresh capture. This is a tool for AI agents to diagnose runtime issues during a session, not a long-term observability system for humans
  #   14. Opt-in per call site via profile_scope!("label") macro at known hot loops only — NOT auto-instrument-all-functions. Rationale: (1) sub-1ns cost when PROFILING_ACTIVE=false means we don't need to be stingy, but covering every fn would balloon the registry with thousands of irrelevant entries that drown out signal during a 10-second window, (2) the goal is diagnosing runaway loops, which are already a small known set (bridge_relay, await_idle, scheduler, forwarding) — instrumenting those plus any new loop added later via PR review is sufficient, (3) opt-in keeps the result payload small so it fits comfortably in a single tool-call response without truncation
  #   15. Flat scopes only — NO parent/child nesting. Rationale: (1) flame-graph nesting adds ~50ns per enter/exit and requires per-thread scope-stack tracking (thread-local storage with push/pop), which is more state to manage and more places for bugs to hide, (2) the diagnostic question we're answering is 'which loop body is spinning' not 'what is the call tree' — flat call_count and total_self_ms answer the spin question directly, (3) the labels are namespaced by module_path!() at compile time (e.g., 'bridge_relay::subordinate_forwarding::recv_loop'), so a human reader can already see the hierarchy from the label string without runtime parent/child tracking, (4) flat scopes serialize trivially as a sorted list, no tree-walking in the response builder
  #
  # EXAMPLES:
  #   1. An AI agent debugging a CPU spike calls AgentManager profile duration_secs=10. The call blocks for 10 seconds while instrumentation is active. The result returns: scopes_by_calls[0] = { label: 'bridge_relay::subordinate_forwarding::recv_loop', call_count: 24_800_000, total_self_ms: 9840, calls_per_sec: 2_480_000, currently_executing_at_end: 9 } — pinpointing the runaway loop spinning at 2.4M calls/sec with 9 concurrent instances at the moment the window closed
  #   2. An AI agent calls AgentManager profile (default duration_secs=10) and the runtime section of the result shows alive_tokio_tasks_start=12 and alive_tokio_tasks_end=47 — the agent immediately sees that 35 tokio tasks accumulated during the 10-second window, confirming a task leak
  #   3. An AI agent narrows in on a suspect module by calling AgentManager profile duration_secs=5 label_prefix='handle_await_idle'. After the 5-second window the result contains only scopes whose label starts with that prefix — letting the agent confirm or rule out the await_idle loop without sifting through hundreds of unrelated entries
  #   4. While an AI agent has a profile session running (started 3 seconds ago, 7 seconds remaining) a second AgentManager profile call from any other agent returns an immediate error response: { error: 'profile_session_active', started_at: '2026-04-07T14:12:30Z', ends_in_secs: 7 } — the second agent waits 7 seconds and retries rather than overlapping windows that would corrupt counters
  #   5. An AI agent calls AgentManager profile duration_secs=10 and inspects the channels section of the result: it sees supervisor_broadcast_<id> with sender_count=1, receiver_count=9, queued_at_end=128, lagged_during_window=42_000 — the agent concludes broadcast subscribers cannot drain fast enough and follows the lead to the subordinate forwarding loop
  #   6. A subordinate LLM agent invokes the AgentManager rig::tool::Tool with action=profile and duration_secs=10 directly via tool_use. It awaits the 10-second blocking response, parses the structured JSON, finds scopes_by_calls[0].currently_executing_at_end=9, and decides to call AgentManager close on the runaway sibling — the entire diagnostic loop happens autonomously without human intervention
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the proc-macro-based instrumentation cover ALL functions in codelet/tools and codelet/napi automatically (via #[instrument] on every fn), or should it be opt-in per call site so we keep the registry small and only profile the loops we care about?
  #   A: Opt-in per call site via profile_scope!("label") macro at known hot loops only — NOT auto-instrument-all-functions. Rationale: (1) sub-1ns cost when PROFILING_ACTIVE=false means we don't need to be stingy, but covering every fn would balloon the registry with thousands of irrelevant entries that drown out signal during a 10-second window, (2) the goal is diagnosing runaway loops, which are already a small known set (bridge_relay, await_idle, scheduler, forwarding) — instrumenting those plus any new loop added later via PR review is sufficient, (3) opt-in keeps the result payload small so it fits comfortably in a single tool-call response without truncation
  #
  #   Q: Should snapshots be persisted to disk (e.g. ~/.fspec/profile-snapshots/<id>.json) so they survive process restarts and can be diffed across runs, or kept in-memory only?
  #   A: No persistence — snapshots are in-memory only, ephemeral, scoped to the AI tool-call lifetime (5-10 second windows). The bounded LRU snapshot store (max 32) lives in process memory and is dropped on restart. This is a tool for AI agents to diagnose runtime issues during a session, not a long-term observability system for humans.
  #
  #   Q: Do you want flame-graph-style parent/child scope nesting (where ProfileScope::enter inside another ProfileScope is recorded as a child), or flat scopes only? Nesting is more powerful but adds ~50ns per enter/exit and complicates the response shape.
  #   A: Flat scopes only — NO parent/child nesting. Rationale: (1) flame-graph nesting adds ~50ns per enter/exit and requires per-thread scope-stack tracking (thread-local storage with push/pop), which is more state to manage and more places for bugs to hide, (2) the diagnostic question we're answering is 'which loop body is spinning' not 'what is the call tree' — flat call_count and total_self_ms answer the spin question directly, (3) the labels are namespaced by module_path!() at compile time (e.g., 'bridge_relay::subordinate_forwarding::recv_loop'), so a human reader can already see the hierarchy from the label string without runtime parent/child tracking, (4) flat scopes serialize trivially as a sorted list, no tree-walking in the response builder
  #
  # ========================================
  Background: User Story
    As a developer or AI agent debugging fspec
    I want to query the AgentManager tool's profile action to get a structured snapshot of the Rust runtime's thread, task, channel, and lock state
    So that I can diagnose CPU/memory anomalies inside the stripped production NAPI binary without needing dtrace, sample(1), or a debug rebuild

  Scenario: Profile a runaway hot loop within a 10-second window
    Given the fspec process has profile_scope! markers compiled into known hot loops
    And no profile session is currently active
    When an AI agent invokes the AgentManager tool with action "profile" and duration_secs 10
    Then the tool call blocks for 10 seconds while instrumentation is active
    And the response contains a scopes_by_calls list sorted by call_count descending
    And the top entry includes label, call_count, total_self_ms, calls_per_sec, max_iter_ms, and currently_executing_at_end fields
    And PROFILING_ACTIVE is reset to false after the response is returned

  Scenario: Detect tokio task leak via runtime metrics during the profile window
    Given the fspec process is running with tokio_unstable enabled at compile time
    When an AI agent invokes the AgentManager profile action with the default duration_secs of 10
    Then the response includes a runtime section with worker_threads, alive_tokio_tasks_start, and alive_tokio_tasks_end fields
    And the response includes a process section with pid, rss_bytes_start, rss_bytes_end, total_threads_start, and total_threads_end fields
    And the difference between alive_tokio_tasks_end and alive_tokio_tasks_start reveals tasks accumulated during the window

  Scenario: Filter scopes by label_prefix to narrow diagnosis to one module
    Given the ProfileRegistry contains scopes from multiple modules including handle_await_idle, bridge_relay, and scheduler
    When an AI agent invokes the AgentManager profile action with duration_secs 5 and label_prefix "handle_await_idle"
    Then the tool call blocks for 5 seconds
    And the scopes_by_calls list contains only entries whose label starts with "handle_await_idle"
    And scopes from bridge_relay and scheduler are excluded from the response

  Scenario: Reject overlapping profile sessions with profile_session_active error
    Given an AI agent has a profile session running with 7 seconds remaining
    When a second AI agent invokes the AgentManager profile action with duration_secs 10
    Then the second call returns immediately without blocking
    And the response contains an error field with value "profile_session_active"
    And the response includes started_at and ends_in_secs fields describing the active session
    And the existing session continues uninterrupted to its scheduled completion

  Scenario: Diagnose channel backpressure via lagged_during_window count
    Given the fspec process has supervisor_broadcast channels wrapped with TrackedBroadcast
    And 9 subordinate agents are subscribed to a single supervisor broadcast channel
    When an AI agent invokes the AgentManager profile action with duration_secs 10
    Then the response includes a channels section listing each registered tracked channel
    And each channel entry contains name, sender_count, receiver_count, queued_at_end, and lagged_during_window fields
    And the supervisor_broadcast entry shows receiver_count of 9 with a non-zero lagged_during_window count

  Scenario: Invoke profile action via rig::tool::Tool from a subordinate LLM agent
    Given a subordinate LLM agent has the AgentManager tool available in its tool catalog
    And the AgentManager JSON schema lists "profile" as a valid action with duration_secs, top_n, and label_prefix parameters
    When the subordinate LLM emits a tool_use call with action "profile" and duration_secs 10
    Then the rig::tool::Tool path dispatches to the same ProfileSession::run as the NAPI handler path
    And the tool result returned to the LLM is structured JSON matching the standard profile result shape

  Scenario: Validate duration_secs is within the allowed 1 to 60 second range
    Given no profile session is currently active
    When an AI agent invokes the AgentManager profile action with duration_secs 0
    Then the call returns immediately with an error indicating duration_secs must be between 1 and 60
    And PROFILING_ACTIVE remains false because the compare-and-swap was never attempted
    When the AI agent retries with duration_secs 61
    Then the call returns immediately with the same out-of-range error

  Scenario: Counters reset to zero at the start of each profile session
    Given an AI agent has just completed a profile session that recorded large counter values
    And PROFILING_ACTIVE has been set back to false
    When the same AI agent invokes the AgentManager profile action with duration_secs 5
    Then the new session reports counter values that reflect only the activity inside the new 5-second window
    And the values from the previous session are not present in the new session's response

  Scenario: Steady-state instrumentation overhead is sub-1ns when no profile session is active
    Given the fspec process has profile_scope! markers compiled into hot loops
    And PROFILING_ACTIVE is false
    When an instrumented hot loop executes for 100,000 iterations
    Then no per-scope counter increments are recorded in the ProfileRegistry
    And the per-iteration overhead is bounded by a single Relaxed atomic load and a not-taken branch
