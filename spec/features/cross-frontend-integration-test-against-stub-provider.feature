@done
@integration
@testing
@rpc
@RPC-066
Feature: Cross-frontend integration test against stub provider
  """
  [A] File layout: (1) NEW rust/fspec/tests/cross_frontend_parity.rs — the integration test binary. (2) NEW rust/fspec/tests/fixtures/cross_frontend_run.jsonl — Rust-pinned golden chunk stream. (3) NEW rust/fspec/tests/README.md — regeneration + future-TS-fixture docs. (4) WIDENED rust/providers/src/stub_provider.rs — adds `impl LlmProvider for StubProvider`. (5) NEW rust/providers/src/stub_provider_registration.rs (or inline in stub_provider.rs) — `pub fn register_stub_provider()` that inserts a synthetic ProviderConfig into the custom provider registry so ProviderType::Custom("stub") routes to it. (6) NEW rust/fspec/Cargo.toml [dev-dependencies] entry `codelet-providers = { workspace = true, features = ["test-support"] }`.
  [B] StubProvider LlmProvider impl: name() = "stub"; model() = "canned"; context_window() = 200_000; max_output_tokens() = 4096; supports_caching() = false; supports_streaming() = true; complete(_) returns Ok("hi back".to_string()); complete_with_tools depends on the scripted run — for inputs containing "trigger-tool" it emits a CompletionResponse with content=MessageContent::ToolUse{...} so the SessionManager's tool dispatcher exercises the ToolCall + ToolResult chunk path; otherwise it emits content=MessageContent::Text("hi back") + StopReason::EndTurn. The tool call is for a sandboxed tool slug `noop_tool` that codelet_tools resolves to a no-op result.
  [C] Custom-provider registration without disk JSON: rather than authoring a `.fspec/providers/stub.json` file (which would force a Rhai-script round-trip), the test calls a NEW helper `codelet_providers::stub_provider::register_stub_provider()` (gated by `test-support`) that synthesises a `codelet_providers::custom::ProviderConfig` and inserts it directly into the custom provider registry. Verify via codelet_providers::custom_provider_registered("stub") == true after registration. This sidesteps Rhai entirely and keeps the test deterministic.
  [D] Daemon boot helper: re-use common::spawn_fspec_daemon (in rust/fspec/tests/common/mod.rs) — same pattern as daemon_mode.rs:37. The test's `mod common;` import gives us spawn_fspec_daemon, make_workspace, ChildGuard, scan_for_port_equals for free. Cross_frontend_parity.rs adds a NEW `setup_stub_workspace(env_vars: &[(&str, &str)]) -> (TempDir, ChildGuard, u16)` helper that (a) calls make_workspace, (b) calls register_stub_provider() in the spawned binary's environment (via FSPEC_RPC_066_FORCE_STUB=1 env var that triggers a one-time registration call inside `codelet_providers::manager::ProviderManager::with_model_support` — only compiled when `test-support` feature is on).
  [E] Subprocess problem: the fspec daemon subprocess is built by `cargo build --bin fspec` which compiles codelet-fspec and its transitive deps WITHOUT the providers/test-support feature (cargo features don't propagate across `[dev-dependencies]` to the binary). Solution: a NEW `[features]` table entry in rust/fspec/Cargo.toml — `test-stub-provider = ["codelet-providers/test-support"]` — and the test invocation becomes `cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity`. The fspec binary built under that feature flag includes the stub provider registration call gated by `#[cfg(feature = "test-stub-provider")]` in src/common.rs (one-time `register_stub_provider()` call inside build_service). Without the feature flag the test compiles into a no-op that prints SKIP and returns Ok (so the default `cargo test` invocation isn't broken).
  [F] Normalisation pipeline: a new module-private `mod normalise` inside cross_frontend_parity.rs exposes `fn normalise_chunk_stream(chunks: &[(SessionId, StreamChunk)]) -> Vec<serde_json::Value>`. For each chunk: (1) serialise to serde_json::Value via serde_json::to_value, (2) recursively walk the Value substituting (a) any string field named `tool_call_id` with "<tc>", (b) any string field named `correlation_id` / `id` matching the UUID regex with "<uuid>", (c) any string field named `timestamp` / `created_at` matching the RFC-3339 regex with "<ts>". Uses regex-once-compiled-via-Lazy. The output Vec<Value> is then serialised line-by-line as JSONL via serde_json::to_string and compared byte-by-byte to the golden file.
  [G] Status-settling discipline: after every backend RPC call that triggers an agent-loop turn (send_input, compact_session), the test calls `wait_for_status(&mut status_rx, SessionStatus::Idle, Duration::from_secs(5))` BEFORE issuing the next RPC. This prevents racey chunk-stream interleaving. The helper subscribes to backend.status_changes_rx() and drains until it sees the target status for the active session.
  [H] Test runtime gating: top-level test fn is annotated `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`. The whole test body is wrapped in `tokio::time::timeout(Duration::from_secs(45), …)` to enforce AC #5 (< 60s). The fixture-regeneration codepath is also wrapped in the same timeout.
  [I] Risk acknowledgement: this is the FIRST card that exercises the real SessionManager + BackgroundSession against a real provider end-to-end through a WS client. Earlier cards (RPC-037, RPC-049, etc.) use StubSessionManagerHandle, which bypasses the agent loop entirely. Expect to discover small bugs in the agent-loop wiring (compaction, interrupt, status-change broadcast on tool calls) during the implementing phase. If the test surfaces blocking bugs, split them into sibling cards rather than fixing inline.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Test file lives at rust/fspec/tests/cross_frontend_parity.rs and exercises the real fspec binary, NOT a MockBackend (that is RPC-065's surface)
  #   2. The test uses daemon mode (fspec daemon --workspace <tmp>) NOT combined mode — combined-mode tests grab /dev/tty via ratatui::init and the existing combined_smoke.rs tests are all #[ignore]'d for that reason; daemon mode exercises the exact same SessionManager + RPC surface (rust/fspec/src/common.rs::build_service is shared) with zero TTY contention
  #   3. The stub provider is a NEW LlmProvider impl in rust/providers/src/stub_provider.rs, gated by the existing `test-support` cargo feature, that returns deterministic responses with NO network I/O — the existing minimal StubProvider struct only exposes canned_chunks() and does not implement LlmProvider; this card widens it
  #   4. The stub provider is registered as a ProviderType::Custom variant via the existing codelet_providers::manager::register_custom_provider mechanism so that SessionManager::create_session_with_id("stub/canned", ...) routes through it without any HTTP requests
  #   5. The scripted run drives every USER-FACING slash command equivalent (/help, plain text, /clear, /thinking high, tool-call-trigger prompt, /compact, /quit) via the WebSocketFspecBackend RPC methods that the AgentView's dispatch path already wires to (NOT by faking keystrokes into the TUI — the TUI is not booted in daemon mode)
  #   6. Captured chunks pass through a normalisation pipeline that substitutes timestamps → <ts>, UUIDs → <uuid>, correlation IDs → <corr>, tool-call IDs → <tc> BEFORE assertions; the normalised stream is byte-identical to spec/attachments/RPC-066/fixtures/ts_reference_run.jsonl (or a pinned Rust-side golden if the TS fixture is deferred — see questions)
  #   7. Total test runtime is under 60 seconds wall-clock; tool calls invoke ONLY sandboxed paths (the stub provider's tool-call response carries a pre-shaped JSON envelope with NO real Bash / file I/O) — the SessionManager's tool dispatcher must never reach codelet_tools::bash or codelet_tools::edit during the scripted run
  #   8. The test fails fast if the stub provider attempts ANY network access (reqwest::get / HTTP / WebSocket leak detection via a deny-network harness wrapper or a tracing assertion that no `eventsource-stream` / `reqwest::Client` paths fire during the scripted run)
  #   9. Regeneration procedure for the TS reference fixture is documented in rust/fspec/tests/README.md (new file) so the team can re-record after a TS frontend change without spelunking the test source
  #   10. Use daemon mode (`fspec daemon --workspace <tmp>`) for headless CI parity — combined and daemon share build_service() so the SessionManager + chunk surface is identical. Combined-mode coverage stays the responsibility of the existing #[ignore]'d combined_smoke.rs tests.
  #   11. Ship a Rust-pinned golden file at rust/fspec/tests/fixtures/cross_frontend_run.jsonl as the initial regression baseline. TS-side fixture regeneration is a follow-up RPC card once the TS stub-provider boot recipe is documented.
  #   12. Widen rust/providers/src/stub_provider.rs to implement the LlmProvider trait behind the existing `test-support` feature; register under slug `stub` via the existing custom-provider mechanism so SessionManager::create_session("stub/canned", ...) reaches it via ProviderType::Custom. This keeps the agent loop in the test path.
  #   13. Omit /thinking from the chunk-stream comparison for this card. Side-effect coverage (set_thinking_level, get_session_model, etc.) lives in the per-method cross-transport parity tests already (rpc037_cross_transport_parity.rs, rpc049_cross_transport_parity.rs, etc.).
  #
  # EXAMPLES:
  #   1. When the test seeds a workspace under tempdir with no spec/work-units.json and spawns `fspec daemon --workspace <tmp>`, scan_for_port_equals on stdout returns a u16 in 1024..=65535 within 5s (mirrors daemon_mode.rs:31 contract)
  #   2. When the workspace contains `.fspec/providers/stub.json` whose `name` is `stub` (handled by codelet_providers::custom::discover_provider_configs), and the test enables the providers crate `test-support` feature so the new StubProvider LlmProvider impl is compiled in, then ProviderType::from_str("stub") returns Ok(ProviderType::Custom("stub")) and SessionManager::create_session("stub/canned", workspace) succeeds without HTTP I/O
  #   3. When the test connects a WebSocketFspecBackend to the daemon, calls backend.create_session("stub/canned"), subscribes to backend.chunks_rx() into a Vec<(SessionId, StreamChunk)>, and calls backend.send_input(session, "hello"), then within 5 seconds the captured vec contains EXACTLY the sequence [Text { text: "hi back", .. }, Done] (matches the existing StubProvider::canned_chunks() contract from RPC-007)
  #   4. When the scripted run sends the sequence [send_input("hello"), clear_history(), set_thinking_level(High), send_input("trigger-tool"), compact_session(), interrupt()] against the daemon, and each step waits for the SessionStatus to settle back to Idle via backend.status_changes_rx(), then the captured chunk stream — after normalisation — is byte-identical to the golden file at rust/fspec/tests/fixtures/cross_frontend_run.jsonl
  #   5. When normalise_chunk_stream(&chunks) is invoked, every Text chunk's text field passes through unchanged, but ToolCall.tool_call_id becomes "<tc>", any UUID matching the regex `[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}` becomes "<uuid>", any RFC-3339 timestamp becomes "<ts>", and any correlation_id field becomes "<corr>"
  #   6. When the rust/fspec/tests/fixtures/cross_frontend_run.jsonl file is missing AND the env var FSPEC_RPC_066_REGENERATE=1 is set, the test re-records the captured-and-normalised stream to that file and skips assertion (exit success); without the env var, a missing fixture fails the test with a clear message pointing at the regeneration command
  #   7. When a regression is injected by editing background_session.rs to emit StreamChunk::text("oops") instead of forwarding the provider chunk, then `cargo test -p codelet-fspec --test cross_frontend_parity` fails with a diff that points at the changed chunk (proves the test catches regressions — AC #4)
  #   8. When `cargo test -p codelet-fspec --test cross_frontend_parity --features codelet-providers/test-support` runs end-to-end, total wall-clock time is under 60s on a stock dev laptop (AC #5)
  #   9. When the test boots the daemon with HTTP_PROXY=http://127.0.0.1:1 and HTTPS_PROXY=http://127.0.0.1:1 (a dead proxy), the stub provider's send_input still resolves to the canned chunk stream within 5s — proving no network egress occurs during the scripted run (deny-network guard)
  #   10. When the new rust/fspec/tests/README.md is read, it contains a section titled `## Regenerating cross_frontend_run.jsonl` describing the FSPEC_RPC_066_REGENERATE=1 step AND a section titled `## Future: TS-recorded reference fixture` documenting the (deferred) follow-up procedure for re-recording chunks from the TS Ink frontend
  #
  # QUESTIONS (ANSWERED):
  #   Q: The card says 'Boot the fspec binary in combined mode'. Combined mode requires /dev/tty (ratatui::init), which is why every combined_smoke.rs test is #[ignore]'d in CI. Should this card (a) follow combined mode and ship #[ignore]'d tests like the existing combined_smoke.rs, OR (b) pivot to daemon mode for headless-CI parity? I propose (b) because both modes share build_service() so the SessionManager + chunk-stream surface under test is identical, and option (b) keeps the test runnable in CI without manual ttyspawn. Confirm?
  #   A: Omit /thinking from the chunk-stream comparison for this card. Side-effect coverage (set_thinking_level, get_session_model, etc.) lives in the per-method cross-transport parity tests already (rpc037_cross_transport_parity.rs, rpc049_cross_transport_parity.rs, etc.).
  #
  #   Q: The card AC #3 demands the Rust chunk stream match a TS reference fixture, with the regeneration procedure being a manual outside-the-test step. Today no such fixture exists. Should this card (a) ship with a TS reference fixture you regenerate as part of this card (requires booting the TS frontend against the same stub provider — significant scope), OR (b) ship with a Rust-side golden file as the initial pinned reference (so the test guards against regressions in the Rust-only chunk stream and the TS-side fixture is added in a follow-up RPC card)? I propose (b) — the Rust frontend has reached structural parity (RPC-029..065) so a Rust-pinned golden is the highest-value asset right now, and a follow-up card can swap it for a TS-recorded fixture once we have a documented stub-provider boot recipe on the TS side.
  #   A: Rust-pinned golden — see rule [10]
  #
  #   Q: For the stub provider, should we (a) build a real LlmProvider impl behind the existing `test-support` feature in rust/providers/src/stub_provider.rs that the SessionManager can reach via ProviderType::Custom("stub"), OR (b) drive the scripted run by directly broadcasting chunks into SharedFspecService::chunks_tx() (bypasses the agent loop entirely — much smaller scope, but loses agent-loop coverage)? I propose (a) — the whole point of the card is end-to-end coverage of the agent loop, so bypassing it via direct broadcast would gut the regression net.
  #   A: LlmProvider impl + ProviderType::Custom — see rule [11]
  #
  #   Q: The scripted run mentions /thinking high → 'ThinkingLevel chunk-like event' but no such StreamChunk variant exists today (set_thinking_level is a backend RPC that mutates session state, not a chunk emitter). Should the test assert (a) on the captured chunks vec only (omitting /thinking from the chunk-stream comparison), OR (b) on a parallel observable-side-effects vec (backend.get_session_model() before/after, get_pending_input changes, etc.) that lives alongside the chunk-stream golden? I propose (a) for this card and (b) as a follow-up if the side-effect surface needs golden-file coverage.
  #   A: Omit /thinking from chunk comparison — see rule [12]
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to run a cross-frontend integration test that drives every slash command end-to-end against the fspec binary backed by a deterministic stub provider
    So that I have a single repeatable regression net proving the Rust frontend's chunk stream matches the TypeScript Ink reference modulo cosmetic differences

  @daemon-boot
  Scenario: fspec daemon boots over a tempworkspace with no work-units and emits a port on stdout
    Given the fspec binary has been built with the test-stub-provider feature enabled
    And a temp workspace exists with an empty spec/work-units.json
    When the test spawns `fspec daemon --workspace <tmp>` as a subprocess
    Then within 5 seconds STDOUT yields a single line parseable as a u16 in 1024..=65535
    And the daemon process remains alive after the port banner is read

  @stub-registration
  Scenario: Workspace registers the stub provider through register_stub_provider()
    Given the fspec binary has been built with the test-stub-provider feature enabled
    And a temp workspace is seeded for daemon-mode boot
    When the daemon's build_service is invoked under cfg(feature = "test-stub-provider")
    Then `codelet_providers::custom_provider_registered("stub")` returns true
    And calling `ProviderType::from_str("stub")` returns Ok(ProviderType::Custom("stub"))
    And the registration occurs exactly once per process

  @hello-canned-stream
  Scenario: send_input("hello") yields the canned [Text, Done] stream within 5 seconds
    Given a running fspec daemon with the stub provider registered
    And an external WebSocketFspecBackend connected to the daemon's port
    And the test has created a session via create_session("stub/canned")
    And the test has subscribed to backend.chunks_rx() into a Vec<(SessionId, StreamChunk)>
    When the test calls backend.send_input(session, "hello")
    Then within 5 seconds the captured vec contains exactly [Text { text: "hi back", .. }, Done]
    And no other chunk variants appear in the vec for that session

  @scripted-run-matches-golden
  Scenario: The full scripted run's normalised chunk stream matches the pinned golden
    Given a running fspec daemon with the stub provider registered
    And the golden file at rust/fspec/tests/fixtures/cross_frontend_run.jsonl exists
    When the test executes the scripted run sequence
      | step | rpc                | argument       |
      | 1    | send_input         | "hello"        |
      | 2    | clear_history      | -              |
      | 3    | set_thinking_level | High           |
      | 4    | send_input         | "trigger-tool" |
      | 5    | compact_session    | -              |
      | 6    | interrupt          | -              |
    And each step waits for SessionStatus::Idle on backend.status_changes_rx() before the next
    Then the captured chunk stream, after normalisation, is byte-identical to the golden file

  @normalisation-pipeline
  Scenario: normalise_chunk_stream substitutes timestamps, UUIDs, correlation IDs, and tool-call IDs
    Given a captured Vec<(SessionId, StreamChunk)> containing a ToolCall, a ToolResult, and a Text chunk
    When normalise_chunk_stream(&chunks) is invoked
    Then every Text chunk's `text` field passes through unchanged
    And every `tool_call_id` field becomes the literal string "<tc>"
    And every UUID matching `[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}` becomes "<uuid>"
    And every RFC-3339 timestamp becomes "<ts>"
    And every `correlation_id` field becomes "<corr>"

  @regeneration-knob
  Scenario: FSPEC_RPC_066_REGENERATE=1 re-records the golden file when it is missing
    Given the file rust/fspec/tests/fixtures/cross_frontend_run.jsonl does not exist
    And the env var FSPEC_RPC_066_REGENERATE is set to "1"
    When `cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity` is run
    Then the test writes the normalised chunk stream to the golden file path
    And the test exits success without asserting against the file
    And re-running the test with FSPEC_RPC_066_REGENERATE unset asserts equality against the freshly-written golden

  @missing-fixture-fails-clearly
  Scenario: Missing golden file fails the test with a clear regeneration hint
    Given the file rust/fspec/tests/fixtures/cross_frontend_run.jsonl does not exist
    And FSPEC_RPC_066_REGENERATE is not set
    When `cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity` is run
    Then the test fails with stderr containing the literal text "FSPEC_RPC_066_REGENERATE=1"
    And the error message names the missing fixture path

  @regression-catch
  Scenario: Injected regression in BackgroundSession's chunk forwarding fails the parity test
    Given the parity test currently passes against the pinned golden
    When a developer edits rust/sessions/src/background_session.rs to substitute StreamChunk::text("oops") for the forwarded provider chunk
    And re-runs `cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity`
    Then the test fails with a diff that names the changed chunk
    And the diff points at the position in the chunk stream where the regression was introduced

  @runtime-budget
  Scenario: Full parity test completes in under 60 seconds wall-clock
    Given the fspec binary has been built with the test-stub-provider feature enabled
    When `cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity` runs end-to-end
    Then total wall-clock time from invocation to exit is under 60 seconds
    And the test body itself is wrapped in `tokio::time::timeout(Duration::from_secs(45), …)`

  @deny-network-egress
  Scenario: Stub provider produces canned chunks even when the network is denied
    Given the test launches the daemon with HTTP_PROXY=http://127.0.0.1:1 and HTTPS_PROXY=http://127.0.0.1:1
    And an external WebSocketFspecBackend connected to the daemon's port
    And the test has created a session via create_session("stub/canned")
    When the test calls backend.send_input(session, "hello")
    Then within 5 seconds the canned [Text, Done] stream is captured
    And no reqwest::Client or eventsource-stream code path fires during the run

  @readme-documents-regen
  Scenario: rust/fspec/tests/README.md documents regeneration and the deferred TS-fixture path
    Given the rust/fspec/tests/README.md file exists after this card
    When the file is read
    Then it contains a section heading exactly "## Regenerating cross_frontend_run.jsonl"
    And that section names the FSPEC_RPC_066_REGENERATE=1 invocation
    And the file contains a section heading exactly "## Future: TS-recorded reference fixture"
    And that section references the follow-up RPC card that will record a TS-side golden
