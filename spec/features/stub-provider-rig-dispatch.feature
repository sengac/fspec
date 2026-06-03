@done
@testing
@integration
@providers
@RPC-069
Feature: Route ProviderType::Custom("stub") through in-memory LlmProvider registry

  """
  Cargo plumbing is already complete: codelet/fspec/Cargo.toml:93-109 defines test-stub-provider with propagation to both codelet-providers/test-support AND codelet-agent-loop/test-support; codelet/agent-loop/Cargo.toml:50-67 defines test-support that pulls codelet-providers/test-support. No Cargo.toml edits are needed by this card — only verification via `cargo metadata`.
  Stub registration is already wired at codelet/fspec/src/common.rs:122-126 under #[cfg(feature = "test-stub-provider")]: build_service() calls register_stub_provider() then manager.set_default_model("stub/canned"). The manager.rs predicate at lines 131-146 already consults is_stub_registered. The only missing link is the agent-loop dispatch arm at agent_loop.rs:880 — currently "stub" falls through to the _ arm (line 966) which scans disk for ~/.fspec/providers/stub.json (none exists) and errors out as "Unsupported provider: stub".
  Lock-step contract: codelet/agent-loop/src/dispatch.rs:112-117 explicitly documents that agent_loop_dispatch_supports_provider MUST stay in lock-step with the match arms in agent_loop.rs:880. Adding the "stub" arm WITHOUT updating the predicate (or vice versa) is a contract violation and will cause future predicate-based regression tests to silently drift.
  StubProvider::create_rig_agent does NOT exist today. The LlmProvider trait at codelet/providers/src/lib.rs does not define create_rig_agent; each concrete provider (ClaudeProvider, OpenAIProvider, CustomProvider) implements its own inherent create_rig_agent. Path A: add StubModel + StubProvider::create_rig_agent so the stub goes through the same Agent<M> + run_agent_stream_with_images pipeline real providers use.
  Golden fixture protocol: codelet/fspec/tests/fixtures/cross_frontend_run.jsonl does not exist yet on HEAD — the scripted-run test's first PASS requires running with FSPEC_RPC_066_REGENERATE=1 (recording), then re-running without the env var (verification).
  """

  Background: User Story
    As a fspec contributor running the cross-frontend parity suite
    I want to boot the fspec binary against the in-memory stub provider and have send_input emit the canned [Text, Done] chunk stream end-to-end without network egress
    So that the four #[ignore]'d cross-frontend parity tests in codelet/fspec/tests/cross_frontend_parity.rs flip green and any future regression in agent-loop stub-provider dispatch is caught automatically

  Scenario: send_input("hello") yields the canned [Text, Done] stream
    Given a running fspec daemon with the stub provider registered
    And an external WebSocketFspecBackend connected to the daemon's port
    And the test has subscribed to backend.chunks_rx() into a Vec<(SessionId, StreamChunk)>
    And the test has created a session via create_session("stub/canned")
    When the test calls backend.send_input(session, "hello")
    Then within 15 seconds the captured vec contains Text("hi back") followed by Done
    And no other chunk variants appear in the vec for that session

  Scenario: fspec daemon boots over a stub-backed workspace and emits a port
    Given the fspec binary has been built with the test-stub-provider feature enabled
    And a temp workspace exists with an empty spec/work-units.json
    When the test spawns `fspec daemon --workspace <tmp>` as a subprocess
    Then within 5 seconds STDOUT yields a single line parseable as a u16 in 1024..=65535
    And the daemon process remains alive after the port banner is read

  Scenario: scripted run's normalised chunk stream matches the pinned golden
    Given a running fspec daemon with the stub provider registered
    And the golden file at codelet/fspec/tests/fixtures/cross_frontend_run.jsonl exists
    When the test executes the scripted run sequence
    And each step waits for SessionStatus::Idle on backend.status_changes_rx() before the next
    Then the captured chunk stream, after normalisation, is byte-identical to the golden file

  Scenario: stub-backed daemon yields canned chunks even when network egress is denied
    Given the test launches the daemon with HTTP_PROXY=http://127.0.0.1:1 and HTTPS_PROXY=http://127.0.0.1:1
    And an external WebSocketFspecBackend connected to the daemon's port
    And the test has created a session via create_session("stub/canned")
    When the test calls backend.send_input(session, "hello")
    Then within 15 seconds the canned [Text, Done] stream is captured
    And no reqwest::Client or eventsource-stream code path fires during the run

  Scenario: agent_loop_dispatch_supports_provider includes the stub arm under test-support
    Given the codelet-agent-loop crate is compiled with --features test-support
    When agent_loop_dispatch_supports_provider("stub") is called
    Then it returns true
    And the predicate stays in lock-step with the agent_loop.rs match arms

