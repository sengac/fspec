@done
@testing
@integration
@providers
@RPC-069
Feature: Route ProviderType::Custom("stub") through in-memory LlmProvider registry
  """
  Cargo plumbing is already complete: rust/fspec/Cargo.toml:93-109 defines test-stub-provider with propagation to both codelet-providers/test-support AND codelet-agent-loop/test-support; rust/agent-loop/Cargo.toml:50-67 defines test-support that pulls codelet-providers/test-support. No Cargo.toml edits are needed by this card — only verification via `cargo metadata`.
  Stub registration is already wired at rust/fspec/src/common.rs:122-126 under #[cfg(feature = "test-stub-provider")]: build_service() calls register_stub_provider() then manager.set_default_model("stub/canned"). The manager.rs predicate at lines 131-146 already consults is_stub_registered. The only missing link is the agent-loop dispatch arm at agent_loop.rs:880 — currently "stub" falls through to the _ arm (line 966) which scans disk for ~/.fspec/providers/stub.json (none exists) and errors out as "Unsupported provider: stub".
  Lock-step contract: rust/agent-loop/src/dispatch.rs:112-117 explicitly documents that agent_loop_dispatch_supports_provider MUST stay in lock-step with the match arms in agent_loop.rs:880. Adding the "stub" arm WITHOUT updating the predicate (or vice versa) is a contract violation and will cause future predicate-based regression tests to silently drift.
  StubProvider::create_rig_agent does NOT exist today. The LlmProvider trait at rust/providers/src/lib.rs does not define create_rig_agent; each concrete provider (ClaudeProvider, OpenAIProvider, CustomProvider) implements its own inherent create_rig_agent. Path A: add StubModel + StubProvider::create_rig_agent so the stub goes through the same Agent<M> + run_agent_stream_with_images pipeline real providers use.
  Golden fixture protocol: rust/fspec/tests/fixtures/cross_frontend_run.jsonl does not exist yet on HEAD — the scripted-run test's first PASS requires running with FSPEC_RPC_066_REGENERATE=1 (recording), then re-running without the env var (verification).
  """

  Background: User Story
    As a fspec contributor running the cross-frontend parity suite
    I want to boot the fspec binary against the in-memory stub provider and have send_input emit the canned [Text, Done] chunk stream end-to-end without network egress
    So that the four #[ignore]'d cross-frontend parity tests in rust/fspec/tests/cross_frontend_parity.rs flip green and any future regression in agent-loop stub-provider dispatch is caught automatically

  Scenario: agent_loop_dispatch_supports_provider includes the stub arm under test-support
    Given the codelet-agent-loop crate is compiled with --features test-support
    When agent_loop_dispatch_supports_provider("stub") is called
    Then it returns true
    And the predicate stays in lock-step with the agent_loop.rs match arms
