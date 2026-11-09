//! Feature: spec/features/agent-loop-tool-registry.feature
//!
//! RPC-083: every provider turn registers the full tool slate through five
//! calls — `gather_mcp_tool_wrappers`, `create_rig_agent`, the
//! `tool_server_handle.add_tool` loop, `set_mcp_tool_server_handle` and
//! `RigAgent::with_default_depth`. BUG-344: the structural checks read the
//! whole `run_with_provider!` macro in dispatch.rs, `run_openai_turn` in
//! agent_loop/provider_openai.rs and `run_custom_turn` in
//! agent_loop/provider_custom.rs; signature witnesses pin the APIs they call.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "provider_sources/support.rs"]
#[macro_use]
mod support;

use support::{assert_in_order, function_body, macro_body, read_source};

// ===========================================================================
// Scenario: run_with_provider! macro body contains the five canonical
//           tool-registry calls in order
// ===========================================================================

#[test]
fn run_with_provider_macro_contains_five_canonical_calls_in_order() {
    // @step Given the source file rust/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When I locate the run_with_provider! macro body with all of its arms
    let body = macro_body(&src, "run_with_provider");

    // @step Then the body contains `codelet_tools::gather_mcp_tool_wrappers($session.id)`
    assert!(body.contains("codelet_tools::gather_mcp_tool_wrappers($session.id)"));

    // @step And the body contains `provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone())`
    assert_in_order(
        body,
        &[
            "provider.create_rig_agent(",
            "$session.id,",
            "role_preamble.as_deref(),",
            "$thinking.clone(),",
        ],
    );

    // @step And the body contains `agent.tool_server_handle.add_tool(wrapper).await` inside a non-empty wrappers guard
    assert_in_order(
        body,
        &[
            "if !mcp_wrappers.is_empty()",
            "agent.tool_server_handle.add_tool(wrapper).await",
        ],
    );

    // @step And the body contains `codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone())`
    assert_in_order(
        body,
        &[
            "codelet_tools::set_mcp_tool_server_handle(",
            "$session.id,",
            "agent.tool_server_handle.clone(),",
        ],
    );

    // @step And the body contains `codelet_core::RigAgent::with_default_depth(agent)`
    assert!(body.contains("codelet_core::RigAgent::with_default_depth(agent)"));

    // @step And those five calls appear in the order: gather → create_rig_agent → add_tool loop → set_mcp_tool_server_handle → with_default_depth
    assert_in_order(body, &CANONICAL_ORDER);
}

/// The five canonical calls, in the order every provider turn makes them.
const CANONICAL_ORDER: [&str; 5] = [
    "codelet_tools::gather_mcp_tool_wrappers(",
    "create_rig_agent(",
    "agent.tool_server_handle.add_tool(wrapper).await",
    "codelet_tools::set_mcp_tool_server_handle(",
    "codelet_core::RigAgent::with_default_depth(agent)",
];

// ===========================================================================
// Scenario: OpenAI turn mirrors the macro body with the same five canonical calls
// ===========================================================================

#[test]
fn openai_turn_contains_five_canonical_calls() {
    // @step Given the source file rust/agent-loop/src/agent_loop/provider_openai.rs
    let src = read_source("agent_loop/provider_openai.rs");

    // @step When I locate the run_openai_turn function body
    let body = function_body(&src, "run_openai_turn");

    // @step Then the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    assert!(body.contains("codelet_tools::gather_mcp_tool_wrappers(session.id)"));

    // @step And the arm contains `provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value)`
    assert_in_order(
        body,
        &[
            "provider.create_rig_agent(",
            "session.id,",
            "role_preamble.as_deref(),",
            "thinking_config_value,",
        ],
    );

    // @step And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    assert!(body.contains("agent.tool_server_handle.add_tool(wrapper).await"));

    // @step And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    assert!(body.contains(
        "codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())"
    ));

    // @step And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`
    assert!(body.contains("codelet_core::RigAgent::with_default_depth(agent)"));
    assert_in_order(body, &CANONICAL_ORDER);
}

// ===========================================================================
// Scenario: Custom-provider turn wraps CustomProvider::create_rig_agent with
//           the four follow-up calls
// ===========================================================================

#[test]
fn custom_provider_turn_contains_four_follow_up_calls() {
    // @step Given the source file rust/agent-loop/src/agent_loop/provider_custom.rs
    let src = read_source("agent_loop/provider_custom.rs");

    // @step When I locate the run_custom_turn function body
    let body = function_body(&src, "run_custom_turn");

    // @step Then the arm calls `codelet_providers::custom::CustomProvider::create_rig_agent`
    assert!(body.contains("codelet_providers::custom::CustomProvider::create_rig_agent("));

    // @step And the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    assert!(body.contains("codelet_tools::gather_mcp_tool_wrappers(session.id)"));

    // @step And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    assert!(body.contains("agent.tool_server_handle.add_tool(wrapper).await"));

    // @step And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    assert!(body.contains(
        "codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())"
    ));

    // @step And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`
    assert_in_order(
        body,
        &[
            "CustomProvider::create_rig_agent(",
            "codelet_tools::gather_mcp_tool_wrappers(",
            "agent.tool_server_handle.add_tool(wrapper).await",
            "codelet_tools::set_mcp_tool_server_handle(",
            "codelet_core::RigAgent::with_default_depth(agent)",
        ],
    );
}

// ===========================================================================
// Scenario: Every provider's create_rig_agent accepts the
//           (session_id, preamble, thinking) signature the macro relies on
// ===========================================================================

#[test]
fn every_provider_create_rig_agent_has_uniform_signature() {
    use codelet_providers::claude::ClaudeProvider;
    use codelet_providers::codex::CodexProvider;
    use codelet_providers::copilot::CopilotProvider;
    use codelet_providers::gemini::GeminiProvider;
    use codelet_providers::openai::OpenAIProvider;
    use codelet_providers::zai::ZAIProvider;

    // @step Given the providers crate exposes ClaudeProvider, OpenAIProvider, GeminiProvider, ZaiProvider, CodexProvider, CopilotProvider
    // (proven by `use` lines above — non-existent providers fail to compile)

    // @step When I take a closure reference to each provider's `create_rig_agent` method
    preamble_signature_witness!(
        ClaudeProvider,
        OpenAIProvider,
        GeminiProvider,
        ZAIProvider,
        CodexProvider,
        CopilotProvider
    );

    // @step Then each closure compiles with parameters `(provider, session_id: uuid::Uuid, preamble: Option<&str>, thinking: Option<serde_json::Value>)`
    // (compile-time witnesses — if any provider's signature drifts this
    //  test fails to compile)

    // @step And the signature is stable across all six built-in providers
    use codelet_agent_loop::agent_loop_dispatch_supports_provider;
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
            agent_loop_dispatch_supports_provider(provider),
            "dispatch predicate must list `{provider}` so the create_rig_agent \
             signature contract is reachable through the agent loop"
        );
    }
}

// ===========================================================================
// Scenario: RigAgent::with_default_depth accepts any rig Agent built by
//           create_rig_agent
// ===========================================================================

#[test]
fn rig_agent_with_default_depth_accepts_provider_built_agents() {
    use rig::agent::Agent;
    use rig::completion::CompletionModel;

    // @step Given codelet_core::RigAgent::with_default_depth has signature `fn(Agent<M>) -> RigAgent<M>`
    // @step When I take a closure reference to RigAgent::with_default_depth
    fn typecheck<M>(agent: Agent<M>) -> codelet_core::RigAgent<M>
    where
        M: CompletionModel + 'static,
    {
        codelet_core::RigAgent::with_default_depth(agent)
    }

    // @step Then the closure compiles for the rig::agent::Agent returned by every provider's create_rig_agent
    //
    // The generic function above proves the signature accepts any
    // `Agent<M>` where `M: CompletionModel + 'static`. Each provider's
    // `create_rig_agent` returns exactly such an `Agent<M>` (verified by
    // the prior scenario's closure witnesses), so the contract holds for
    // every dispatched provider in turn.
    let _witness: fn(
        Agent<rig::providers::openai::CompletionModel>,
    ) -> codelet_core::RigAgent<rig::providers::openai::CompletionModel> = typecheck;
}

// ===========================================================================
// Scenario: gather_mcp_tool_wrappers and set_mcp_tool_server_handle have
//           stable session-id signatures
// ===========================================================================

#[test]
fn mcp_helper_signatures_are_stable() {
    use codelet_tools::{gather_mcp_tool_wrappers, set_mcp_tool_server_handle};

    // @step Given codelet_tools exposes `gather_mcp_tool_wrappers` and `set_mcp_tool_server_handle`
    // (proven by `use` lines above)

    // @step When I take closure references to both functions
    // @step Then gather_mcp_tool_wrappers compiles as `fn(uuid::Uuid) -> Vec<McpToolWrapper>`
    let _gather: fn(uuid::Uuid) -> Vec<codelet_tools::McpToolWrapper> = gather_mcp_tool_wrappers;

    // @step And set_mcp_tool_server_handle compiles as `fn(uuid::Uuid, ToolServerHandle)`
    let _set: fn(uuid::Uuid, rig::tool::server::ToolServerHandle) = set_mcp_tool_server_handle;
}

// ===========================================================================
// Scenario: gather_mcp_tool_wrappers returns an empty Vec when no MCP
//           servers are connected for the session
// ===========================================================================

#[test]
fn gather_mcp_tool_wrappers_returns_empty_vec_for_unknown_session() {
    use uuid::Uuid;

    // @step Given a freshly minted session id with no MCP servers connected
    let session_id = Uuid::new_v4();

    // @step When I call `codelet_tools::gather_mcp_tool_wrappers(session_id)`
    let wrappers = codelet_tools::gather_mcp_tool_wrappers(session_id);

    // @step Then the returned vector is empty
    assert!(
        wrappers.is_empty(),
        "fresh session must have no MCP wrappers; got {} entries",
        wrappers.len()
    );

    // @step And the function is callable from the codelet-agent-loop crate's test scope
    // (proven by the call above compiling in this test crate)
}
