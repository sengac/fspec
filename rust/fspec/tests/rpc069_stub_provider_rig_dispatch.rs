//! Stub-provider dispatch in codelet-agent-loop (RPC-069).
//!
//! Feature: spec/features/stub-provider-rig-dispatch.feature
//!
//! Moved out of cross_frontend_parity.rs so each test file maps one feature
//! (COV-075). The daemon, send_input, golden and network-denied scenarios
//! RPC-069 shared with RPC-066 were merged into
//! cross-frontend-integration-test-against-stub-provider.feature.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// ---------------------------------------------------------------------
// Scenario: agent_loop_dispatch_supports_provider includes the stub arm
// under test-support (RPC-069 lock-step contract)
// ---------------------------------------------------------------------

#[cfg(feature = "test-stub-provider")]
#[test]
fn scenario_agent_loop_dispatch_supports_stub_arm() {
    // @step Given the codelet-agent-loop crate is compiled with --features test-support
    // (Compiled when cargo test runs codelet-fspec with --features
    // test-stub-provider, which propagates test-support into
    // codelet-agent-loop per Cargo.toml line 105-109.)

    // @step When agent_loop_dispatch_supports_provider("stub") is called
    let supports_stub = codelet_agent_loop::agent_loop_dispatch_supports_provider("stub");

    // @step Then it returns true
    assert!(
        supports_stub,
        "agent_loop_dispatch_supports_provider(\"stub\") must return true under test-support \
         (lock-step with the \"stub\" => arm in agent_loop.rs)"
    );

    // @step And the predicate stays in lock-step with the agent_loop.rs match arms
    // (Existing arms must remain supported. If any of these regress,
    // the dispatch.rs contract has drifted from the match block in
    // agent_loop.rs.)
    for provider in [
        "claude",
        "openai",
        "gemini",
        "zai",
        "codex",
        "github-copilot",
        "copilot",
    ] {
        assert!(
            codelet_agent_loop::agent_loop_dispatch_supports_provider(provider),
            "existing arm '{provider}' must remain supported alongside the stub arm"
        );
    }
}
