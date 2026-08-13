@done
@session-management
@codelet
@refactor
@rust
@infrastructure
@napi
@RPC-043
Feature: Reduce codelet-napi to thin adapter (session_bindings.rs); update Cargo.toml

  # Discovery (2026-05-21): the live rust/napi/src/session_manager.rs is 6752 LOC
  # (not the 8645 quoted in the original RPC-043 attachment). The drift since 2026-05-19
  # is consistent with the line-offset drift documented in RPC-040 review-findings.
  #
  # Decision (2026-05-21, user input): the non-#[napi] helpers DO NOT collapse into
  # session_bindings.rs. They split into seven sibling modules under rust/napi/src/:
  # session_bindings.rs (thin #[napi] adapters only), agent_loop.rs, persist.rs,
  # footer_poller.rs, bridges.rs, session_hooks.rs, interjection.rs. Pre-existing
  # #[cfg(test)] modules migrate with the code they exercise. session_manager.rs is
  # deleted at the end.
  #
  # Decision (2026-05-21, user input): the Cargo.toml audit interpretation is
  # realistic — keep direct deps on codelet-cli/providers/git/tools because the
  # napi-side helpers still use them via ~120 call sites; annotate each entry with
  # an inline RPC-043 comment explaining the consumer module; defer aggressive
  # dep pruning to a follow-up card after RPC-068 lands.
  #
  # Behaviour preservation: rust/napi/index.d.ts diff is EMPTY post-RPC-043;
  # every existing test passes against its new file location; a new shape test
  # (rust/napi/tests/session_bindings_shape.rs) enforces the static contract;
  # a new smoke test (rust/napi/tests/session_bindings_smoke.rs) exercises every
  # napi wrapper at least once.
  #
  # See: spec/attachments/RPC-043/reduce-napi-to-thin-adapter.md (original spec)
  #      spec/attachments/RPC-043/review-findings.md (discoveries + decisions)
  Background: User Story
    As a Rust developer maintaining the codelet workspace
    I want to reduce codelet-napi/src/session_manager.rs to a thin set of #[napi] adapter wrappers in a new session_bindings.rs module, with non-NAPI helpers split into sibling modules
    So that the codelet-napi crate becomes a pure JS-bridge adapter with no agent logic of its own — the agent loop, hooks, and session manager live in codelet-sessions, while napi-internal infrastructure (footer poller, bridges, persistence shims) is properly modularised, unblocking the fspec binary wiring in RPC-044 and keeping the rust/napi/index.d.ts byte-stable for the TS frontend

  @rule:session_manager_deleted
  @structure
  Scenario: codelet-napi/src/session_manager.rs no longer exists after the move
    Given the codelet workspace has the RPC-042 changes landed on main
    When I run `ls rust/napi/src/session_manager.rs`
    Then the command exits with a non-zero status
    And stderr contains "No such file or directory"
    And the file at "rust/napi/src/session_manager.rs" does not exist on disk

  @rule:seven_sibling_modules
  @structure
  Scenario: codelet-napi/src/ gains the seven new sibling modules
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `ls rust/napi/src/`
    Then the listing includes "session_bindings.rs"
    And the listing includes "agent_loop.rs"
    And the listing includes "persist.rs"
    And the listing includes "footer_poller.rs"
    And the listing includes "bridges.rs"
    And the listing includes "session_hooks.rs"
    And the listing includes "interjection.rs"

  @rule:lib_rs_module_declarations
  @structure
  Scenario: lib.rs declares each new sibling module under the noop feature gate
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/lib.rs`
    Then the file declares `pub mod session_bindings;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod agent_loop;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod persist;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod footer_poller;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod bridges;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod session_hooks;` under `#[cfg(not(feature = "noop"))]`
    And the file declares `pub mod interjection;` under `#[cfg(not(feature = "noop"))]`
    And the old `pub mod session_manager;` line is absent
    And the file contains `pub use session_bindings::*;` under `#[cfg(not(feature = "noop"))]`

  @rule:session_bindings_shape
  @napi
  @thin_adapter
  Scenario: session_bindings.rs holds 66 to 68 #[napi] free functions and the 12 #[napi(object)] shapes
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/session_bindings.rs`
    Then the file declares 66 to 68 free functions decorated with `#[napi]` (counted by `^#\[napi\]\n(?:pub )?(?:async )?fn `)
    And the file declares the 12 `#[napi(object)]` structs: GlobalChunkCallbackArgs, IsolatedSessionResult, SessionModel, SessionTokens, NapiPauseState, SupervisorRoleInfo, JsWorkUnitContext, PathValidationResult, BashExecutionResult, JsProviderModelInfo, JsProviderInfo, JsProviderTestResult
    And the file contains 0 occurrences of the literal string `async fn agent_loop`
    And the file contains 0 occurrences of the literal string `impl codelet_sessions::session_manager::SessionManagerHooks`
    And the file size is between 2500 and 4000 lines of code

  @rule:agent_loop_owns_loop
  @agent_loop
  Scenario: agent_loop.rs owns the agent loop and its StreamOutput sinks
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/agent_loop.rs`
    Then the file declares `pub(crate) async fn agent_loop(session: Arc<BackgroundSession>, input_rx: mpsc::Receiver<PromptInput>, mcp_injection_rx: mpsc::Receiver<McpInjection>)`
    And the file declares the `InputWithImages` helper struct
    And the file declares the `BackgroundOutput` struct
    And the file declares `impl codelet_cli::interactive::StreamOutput for BackgroundOutput`
    And the file declares the `BackgroundProgressEmitter` struct
    And the file declares `impl codelet_cli::interactive::StreamOutput for BackgroundProgressEmitter`
    And the file contains a `#[cfg(test)] mod agent_loop_dispatch_tests` section at the bottom
    And no `#[napi]` or `#[napi(object)]` attribute appears anywhere in the file

  @rule:session_hooks_owns_hooks
  @hooks
  Scenario: session_hooks.rs owns the NapiSessionManagerHooks impl and the install helper
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/session_hooks.rs`
    Then the file declares `pub struct NapiSessionManagerHooks`
    And the file contains `impl codelet_sessions::session_manager::SessionManagerHooks for NapiSessionManagerHooks`
    And the impl block contains six methods: spawn_agent_loop, spawn_scheduler, ensure_scheduler_running_for_loop, spawn_footer_poller, stop_footer_poller, cleanup_session_loops
    And spawn_agent_loop invokes `crate::agent_loop::agent_loop(...)`
    And spawn_scheduler invokes `crate::scheduler::spawn_scheduler(...)`
    And spawn_footer_poller invokes `crate::footer_poller::spawn_footer_poller(...)`
    And stop_footer_poller invokes `crate::footer_poller::stop_footer_poller(...)`
    And cleanup_session_loops invokes `crate::scheduler::LoopStore::instance().remove_for_session(...)`
    And the file declares `pub(crate) fn install_napi_session_manager_hooks()` at the bottom

  @rule:persist_rs_owns_persisters
  @persistence
  Scenario: persist.rs owns the five persist_* helpers as pub(crate) functions
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/persist.rs`
    Then the file declares `pub(crate) fn persist_user_message`
    And the file declares `pub(crate) fn persist_assistant_message_internal`
    And the file declares `pub(crate) fn persist_tool_result_internal`
    And the file declares `pub(crate) fn persist_token_state`
    And the file declares `pub(crate) fn persist_pending_annotations`
    And no `#[napi]` attribute appears in the file

  @rule:footer_poller_rs
  @footer_poller
  Scenario: footer_poller.rs owns the FOOTER_POLLER_TOKENS static and the spawn/stop helpers
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/footer_poller.rs`
    Then the file declares the `FOOTER_POLLER_TOKENS` static via `once_cell::sync::Lazy`
    And the file declares `pub(crate) fn spawn_footer_poller(session_id: String, cwd: String, worktree_path: Option<String>)`
    And the file declares `pub(crate) fn stop_footer_poller(session_id: &str)`
    And the `FOOTER_POLLER_TOKENS` static is private to the module (not `pub`)

  @rule:bridges_rs
  @bridges
  Scenario: bridges.rs owns the bridge init helpers and the handler registration helpers
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/bridges.rs`
    Then the file declares `pub(crate) fn init_block_notification_callbacks`
    And the file declares `pub(crate) fn init_bridge_metadata_providers`
    And the file declares `pub(crate) fn init_bridge_session_and_terminal_creators`
    And the file declares `pub(crate) fn emit_block_notification_to_tui`
    And the file declares `pub(crate) fn register_deep_search_handler`
    And the file declares `pub(crate) fn register_agent_manager_handler`
    And the file contains `#[cfg(test)]` companions for supervisor_broadcast_tests, global_chunk_callback_tests, is_attached_gating_tests, correlation_id_tests

  @rule:interjection_rs
  @interjection
  Scenario: interjection.rs owns the parse_interjection function and the Interjection struct
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/src/interjection.rs`
    Then the file declares the `Interjection` struct as `pub(crate)` (or pub) with `urgent: bool` and `content: String`
    And the file declares `pub(crate) fn parse_interjection(response: &str) -> Option<Interjection>`

  @rule:tests_migration
  @tests
  Scenario: pre-existing #[cfg(test)] modules migrate to their owning sibling modules without losing assertions
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo test -p codelet-napi`
    Then chain_of_command_tests passes (already in codelet-sessions via RPC-040 re-export)
    And supervisor_broadcast_tests passes from inside bridges.rs
    And is_attached_gating_tests passes from inside bridges.rs
    And global_chunk_callback_tests passes from inside bridges.rs
    And session_role_tests passes from inside session_bindings.rs
    And supervisor_loop_tests passes from inside session_bindings.rs
    And supervisor_input_tests passes from inside session_bindings.rs
    And napi_supervisor_tests passes from inside session_bindings.rs
    And correlation_id_tests passes from inside bridges.rs
    And supervisor_integration_tests passes from inside session_bindings.rs
    And work_unit_context_tests passes from inside session_bindings.rs
    And agent_loop_dispatch_tests passes from inside agent_loop.rs
    And sub_agent_model_inheritance_tests passes from inside session_bindings.rs
    And bug132_tests passes from inside session_bindings.rs

  @rule:napi_build_default
  @build
  Scenario: codelet-napi builds with default features
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo build -p codelet-napi`
    Then the build succeeds with exit code 0
    And no compilation warning mentions `crate::session_manager`

  @rule:napi_build_noop
  @build
  Scenario: codelet-napi builds with the noop feature
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo build -p codelet-napi --features noop`
    Then the build succeeds with exit code 0
    And the seven new sibling modules are excluded from the build by the `#[cfg(not(feature = "noop"))]` gate in lib.rs

  @rule:index_dts_byte_stable
  @ts_contract
  Scenario: rust/napi/index.d.ts is byte-identical to the pre-RPC-043 baseline
    Given the RPC-043 changes are applied to the codelet workspace
    And the pre-RPC-043 baseline of `rust/napi/index.d.ts` is committed
    When I run `git diff rust/napi/index.d.ts`
    Then no removed export lines appear in the diff (NAPI ABI preserved)
    And `sessionManagerCreate(model: string, project: string): Promise<string>;` is exported unchanged
    And `sessionSetGlobalChunkCallback(callback: (args: GlobalChunkCallbackArgs) => void): void;` is exported unchanged
    And `interface GlobalChunkCallbackArgs { sessionId: string; chunk: StreamChunk }` preserves field order
    And `interface IsolatedSessionResult { sessionId: string; worktreePath: string; baseCommit: string }` preserves field order

  @rule:napi_test_default
  @test
  Scenario: cargo test -p codelet-napi passes
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo check --tests -p codelet-napi`
    Then the command exits with code 0
    And no test from the pre-RPC-043 baseline is reported as missing
    And every test target compiles after the module migration

  @rule:sessions_test_unchanged
  @regression
  Scenario: cargo test -p codelet-sessions continues to pass
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo check --tests -p codelet-sessions`
    Then the command exits with code 0
    And the existing RPC-038/039/040/041/042 shape and smoke tests all pass

  @rule:shape_test_added
  @test
  @structure
  Scenario: A new shape test enforces the static structural contract
    Given the RPC-043 changes are applied to the codelet workspace
    When I run `cargo test -p codelet-napi --test session_bindings_shape`
    Then the command exits with code 0
    And the test asserts `rust/napi/src/session_manager.rs` does not exist
    And the test asserts each of session_bindings.rs / agent_loop.rs / persist.rs / footer_poller.rs / bridges.rs / session_hooks.rs / interjection.rs exists
    And the test asserts lib.rs declares each new sibling module
    And the test asserts lib.rs contains `pub use session_bindings::*;`
    And the test asserts session_bindings.rs contains exactly 66 `#[napi]` free-function declarations
    And the test asserts NapiSessionManagerHooks impl lives in session_hooks.rs, not session_bindings.rs
    And the test asserts the agent_loop function definition lives in agent_loop.rs, not session_bindings.rs

  @rule:smoke_test_added
  @test
  @behaviour
  @rule:cargo_toml_annotated
  @cargo
  @manifest
  Scenario: rust/napi/Cargo.toml carries inline RPC-043 comments naming consumer modules
    Given the RPC-043 changes are applied to the codelet workspace
    When I open `rust/napi/Cargo.toml`
    Then `codelet-sessions = { path = "../sessions" }` is declared as a dependency
    And the `codelet-cli` entry carries an inline `# RPC-043:` comment naming its consumer modules (agent_loop.rs, persist.rs, bridges.rs, session_bindings.rs)
    And the `codelet-providers` entry carries an inline `# RPC-043:` comment naming its consumer modules (session_bindings.rs, bridges.rs, agent_loop.rs)
    And the `codelet-git` entry carries an inline `# RPC-043:` comment naming its consumer modules (agent_loop.rs, bridges.rs, footer_poller.rs)
    And the `codelet-tools` entry carries an inline `# RPC-043:` comment naming its consumer modules (agent_loop.rs, session_bindings.rs, bridges.rs, footer_poller.rs, session_hooks.rs)
    And the `codelet-core` entry carries an inline `# RPC-043:` comment naming its consumer modules (agent_loop.rs, persist.rs)
    And the `codelet-common` entry carries an inline `# RPC-043:` comment naming its consumer modules (session_bindings.rs)
    And no codelet-* direct dependency is removed in this card

  @rule:behaviour_preserved
  @behaviour
  Scenario: Dependent shape tests retarget to session_bindings.rs after session_manager.rs deletion
    Given rust/sessions/tests/background_session_shape.rs was authored by RPC-039 and extended by RPC-041 to grep rust/napi/src/session_manager.rs for re-export and GLOBAL_CHUNK_CALLBACK invariants
    When I run `cargo test --release -p codelet-sessions --test background_session_shape` against the post-RPC-043 worktree
    Then every test in background_session_shape.rs reports `ok` and the binary exits 0
    Given RPC-043 deletes rust/napi/src/session_manager.rs and moves its re-exports and #[napi] wrappers verbatim into rust/napi/src/session_bindings.rs
    Then the `napi_shell_path()` helper in background_session_shape.rs resolves to rust/napi/src/session_bindings.rs and that file exists on disk
    Then the 6 previously-broken scenarios (codelet_napi_still_builds_against_the_re_exported_background_session, send_input_is_rewritten_to_a_non_napi_result_type, handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback, pre_existing_in_file_unit_tests_in_codelet_napi_still_pass_via_the_re_exports, napi_typescript_surface_is_byte_stable_across_the_move, global_chunk_callback_static_struct_and_unsafe_impls_are_removed) all pass by reading session_bindings.rs and observing the same invariants previously observed against session_manager.rs (6 pub use background_session re-exports, 17 pub fn session_* free functions, zero GLOBAL_CHUNK_CALLBACK references, GlobalChunkCallbackArgs struct present)
