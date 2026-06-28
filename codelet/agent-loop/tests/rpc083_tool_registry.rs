//! Feature: spec/features/agent-loop-tool-registry.feature
//!
//! RPC-083 (child of RPC-072 family): the canonical NAPI agent loop
//! (`codelet/napi/src/agent_loop.rs:79-120` macro body + inlined arms)
//! registers the full tool slate through five calls inside each provider
//! dispatch path:
//!
//!   1. `codelet_tools::gather_mcp_tool_wrappers($session.id)`
//!   2. `provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone())`
//!   3. `agent.tool_server_handle.add_tool(wrapper).await` (loop)
//!   4. `codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone())`
//!   5. `codelet_core::RigAgent::with_default_depth(agent)`
//!
//! After the RPC-080/081/082 ports the same plumbing now lives in the
//! NAPI-free `codelet-agent-loop` crate. This file pins the RPC-083
//! contract across three dispatch paths (`run_with_provider!` macro,
//! inlined "openai" match arm, custom-provider fallthrough) plus
//! signature witnesses for `create_rig_agent`,
//! `RigAgent::with_default_depth`, `gather_mcp_tool_wrappers`, and
//! `set_mcp_tool_server_handle`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers — locate the agent-loop source files relative to CARGO_MANIFEST_DIR.
// ===========================================================================

fn agent_loop_src(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file)
}

fn read_source(file: &str) -> String {
    let path = agent_loop_src(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract the body of the `run_with_provider!` macro_rules! macro from
/// `dispatch.rs`. Returns the substring spanning the outer `{ ... }` of
/// the single matcher arm.
fn extract_run_with_provider_macro_body(src: &str) -> &str {
    let macro_start = src
        .find("macro_rules! run_with_provider")
        .expect("dispatch.rs must define `macro_rules! run_with_provider`");
    let tail = &src[macro_start..];
    let arrow = tail
        .find("=> {")
        .expect("run_with_provider! must have a single `=> {` arm");
    let body_start = macro_start + arrow + "=> ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for run_with_provider! macro body");
}

/// Extract the inlined `"openai" => { ... }` match arm body from
/// `agent_loop.rs`. The arm is inlined (not a macro expansion) because
/// `get_openai` requires `session.id` for cache-optimisation headers.
fn extract_openai_match_arm(src: &str) -> &str {
    let arm_start = src
        .find("\"openai\" => {")
        .expect("agent_loop.rs must have an inlined `\"openai\" => {` arm");
    let body_start = arm_start + "\"openai\" => ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for \"openai\" => arm body");
}

/// Extract the custom-provider fallthrough arm body — the `_ => { ... }`
/// inside the provider match in `agent_loop.rs`. Narrowed to the region
/// that contains `CustomProvider::create_rig_agent` so unrelated `_ =>`
/// branches elsewhere in the file do not match.
fn extract_custom_provider_fallthrough_arm(src: &str) -> &str {
    let create_call = src
        .find("CustomProvider::create_rig_agent")
        .expect("agent_loop.rs must call CustomProvider::create_rig_agent");
    let prefix = &src[..create_call];
    let arm_marker = prefix
        .rfind("_ => {")
        .expect("CustomProvider::create_rig_agent must live inside a `_ => {` arm");
    let body_start = arm_marker + "_ => ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for `_ =>` fallthrough arm body");
}

/// Find the byte offset of `needle` in `haystack`, returning None if absent.
fn find_pos(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

// ===========================================================================
// Scenario: run_with_provider! macro body contains the five canonical
//           tool-registry calls in order
// ===========================================================================

#[test]
fn run_with_provider_macro_contains_five_canonical_calls_in_order() {
    // @step Given the source file codelet/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When I locate the run_with_provider! macro body
    let body = extract_run_with_provider_macro_body(&src);

    // @step Then the body contains `codelet_tools::gather_mcp_tool_wrappers($session.id)`
    let gather_pos = find_pos(body, "codelet_tools::gather_mcp_tool_wrappers($session.id)")
        .expect("macro body must call `codelet_tools::gather_mcp_tool_wrappers($session.id)`");

    // @step And the body contains `provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone())`
    let create_pos = find_pos(body, "provider.create_rig_agent(")
        .expect("macro body must invoke `provider.create_rig_agent(`");
    assert!(
        body[create_pos..].contains("$session.id"),
        "create_rig_agent first arg must be `$session.id`; body=\n{body}"
    );
    assert!(
        body[create_pos..].contains("role_preamble.as_deref()"),
        "create_rig_agent must pass `role_preamble.as_deref()`; body=\n{body}"
    );
    assert!(
        body[create_pos..].contains("$thinking.clone()"),
        "create_rig_agent must pass `$thinking.clone()`; body=\n{body}"
    );

    // @step And the body contains `agent.tool_server_handle.add_tool(wrapper).await` inside a non-empty wrappers guard
    let add_tool_pos = find_pos(body, "agent.tool_server_handle.add_tool(wrapper).await")
        .expect("macro body must call `agent.tool_server_handle.add_tool(wrapper).await`");
    assert!(
        body.contains("if !mcp_wrappers.is_empty()"),
        "add_tool loop must be guarded by `if !mcp_wrappers.is_empty()`; body=\n{body}"
    );
    let guard_pos = find_pos(body, "if !mcp_wrappers.is_empty()")
        .expect("non-empty guard must precede add_tool call");
    assert!(
        guard_pos < add_tool_pos,
        "non-empty wrappers guard must come before add_tool call"
    );

    // @step And the body contains `codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone())`
    let set_handle_pos = find_pos(body, "codelet_tools::set_mcp_tool_server_handle(")
        .expect("macro body must call `codelet_tools::set_mcp_tool_server_handle(`");
    assert!(
        body[set_handle_pos..].contains("$session.id"),
        "set_mcp_tool_server_handle first arg must be `$session.id`"
    );
    assert!(
        body[set_handle_pos..].contains("agent.tool_server_handle.clone()"),
        "set_mcp_tool_server_handle second arg must be `agent.tool_server_handle.clone()`"
    );

    // @step And the body contains `codelet_core::RigAgent::with_default_depth(agent)`
    let with_depth_pos = find_pos(body, "codelet_core::RigAgent::with_default_depth(agent)")
        .expect(
            "macro body must wrap the rig agent with \
             `codelet_core::RigAgent::with_default_depth(agent)`",
        );

    // @step And those five calls appear in the order: gather → create_rig_agent → add_tool loop → set_mcp_tool_server_handle → with_default_depth
    assert!(
        gather_pos < create_pos,
        "gather_mcp_tool_wrappers must precede create_rig_agent"
    );
    assert!(
        create_pos < add_tool_pos,
        "create_rig_agent must precede add_tool loop"
    );
    assert!(
        add_tool_pos < set_handle_pos,
        "add_tool loop must precede set_mcp_tool_server_handle"
    );
    assert!(
        set_handle_pos < with_depth_pos,
        "set_mcp_tool_server_handle must precede with_default_depth"
    );
}

// ===========================================================================
// Scenario: OpenAI inlined arm in agent_loop.rs mirrors the macro body
//           with the same five canonical calls
// ===========================================================================

#[test]
fn openai_inlined_arm_contains_five_canonical_calls() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When I locate the "openai" => match arm body
    let body = extract_openai_match_arm(&src);

    // @step Then the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    let gather_pos = find_pos(body, "codelet_tools::gather_mcp_tool_wrappers(session.id)")
        .expect("openai arm must call `codelet_tools::gather_mcp_tool_wrappers(session.id)`");

    // @step And the arm contains `provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value.clone())`
    let create_pos = find_pos(body, "provider.create_rig_agent(")
        .expect("openai arm must invoke `provider.create_rig_agent(`");
    assert!(
        body[create_pos..].contains("session.id"),
        "openai create_rig_agent first arg must be `session.id`"
    );
    assert!(
        body[create_pos..].contains("role_preamble.as_deref()"),
        "openai create_rig_agent must pass `role_preamble.as_deref()`"
    );
    assert!(
        body[create_pos..].contains("thinking_config_value.clone()"),
        "openai create_rig_agent must pass `thinking_config_value.clone()`"
    );

    // @step And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    let add_tool_pos = find_pos(body, "agent.tool_server_handle.add_tool(wrapper).await")
        .expect("openai arm must call `agent.tool_server_handle.add_tool(wrapper).await`");

    // @step And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    let set_handle_pos = find_pos(body, "codelet_tools::set_mcp_tool_server_handle(")
        .expect("openai arm must call `codelet_tools::set_mcp_tool_server_handle(`");
    assert!(
        body[set_handle_pos..].contains("session.id"),
        "openai set_mcp_tool_server_handle first arg must be `session.id`"
    );
    assert!(
        body[set_handle_pos..].contains("agent.tool_server_handle.clone()"),
        "openai set_mcp_tool_server_handle second arg must be `agent.tool_server_handle.clone()`"
    );

    // @step And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`
    let with_depth_pos = find_pos(body, "codelet_core::RigAgent::with_default_depth(agent)")
        .expect(
            "openai arm must wrap the rig agent with \
             `codelet_core::RigAgent::with_default_depth(agent)`",
        );

    // Pin ordering of all five canonical calls.
    assert!(
        gather_pos < create_pos,
        "gather must precede create_rig_agent"
    );
    assert!(
        create_pos < add_tool_pos,
        "create_rig_agent must precede add_tool"
    );
    assert!(
        add_tool_pos < set_handle_pos,
        "add_tool must precede set_handle"
    );
    assert!(
        set_handle_pos < with_depth_pos,
        "set_handle must precede with_default_depth"
    );
}

// ===========================================================================
// Scenario: Custom-provider fallthrough arm wraps
//           CustomProvider::create_rig_agent with the four follow-up calls
// ===========================================================================

#[test]
fn custom_provider_fallthrough_arm_contains_four_follow_up_calls() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When I locate the `_ =>` fallthrough match arm body
    let body = extract_custom_provider_fallthrough_arm(&src);

    // @step Then the arm calls `codelet_providers::custom::CustomProvider::create_rig_agent`
    let create_pos = find_pos(
        body,
        "codelet_providers::custom::CustomProvider::create_rig_agent",
    )
    .expect(
        "custom-provider arm must call \
         `codelet_providers::custom::CustomProvider::create_rig_agent`",
    );

    // @step And the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    let gather_pos = find_pos(body, "codelet_tools::gather_mcp_tool_wrappers(session.id)")
        .expect("custom arm must call `gather_mcp_tool_wrappers(session.id)`");

    // @step And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    let add_tool_pos = find_pos(body, "agent.tool_server_handle.add_tool(wrapper).await")
        .expect("custom arm must call `agent.tool_server_handle.add_tool(wrapper).await`");

    // @step And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    let set_handle_pos = find_pos(body, "codelet_tools::set_mcp_tool_server_handle(")
        .expect("custom arm must call `codelet_tools::set_mcp_tool_server_handle(`");
    assert!(
        body[set_handle_pos..].contains("session.id"),
        "custom set_mcp_tool_server_handle first arg must be `session.id`"
    );
    assert!(
        body[set_handle_pos..].contains("agent.tool_server_handle.clone()"),
        "custom set_mcp_tool_server_handle second arg must be `agent.tool_server_handle.clone()`"
    );

    // @step And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`
    let with_depth_pos = find_pos(body, "codelet_core::RigAgent::with_default_depth(agent)")
        .expect(
            "custom arm must wrap the rig agent with \
             `codelet_core::RigAgent::with_default_depth(agent)`",
        );

    // Order: create_rig_agent → gather → add_tool → set_handle → with_default_depth.
    // (Custom-provider arm performs `CustomProvider::create_rig_agent` *before*
    // the gather, because the wrappers attach to `handle.into_inner()` not to
    // a fresh provider object.)
    assert!(
        create_pos < gather_pos,
        "CustomProvider::create_rig_agent must precede gather_mcp_tool_wrappers in custom arm"
    );
    assert!(
        gather_pos < add_tool_pos,
        "gather must precede add_tool in custom arm"
    );
    assert!(
        add_tool_pos < set_handle_pos,
        "add_tool must precede set_handle in custom arm"
    );
    assert!(
        set_handle_pos < with_depth_pos,
        "set_handle must precede with_default_depth in custom arm"
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
    let _claude = |p: &ClaudeProvider,
                   session_id: uuid::Uuid,
                   preamble: Option<&str>,
                   thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

    let _openai = |p: &OpenAIProvider,
                   session_id: uuid::Uuid,
                   preamble: Option<&str>,
                   thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

    let _gemini = |p: &GeminiProvider,
                   session_id: uuid::Uuid,
                   preamble: Option<&str>,
                   thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

    let _zai = |p: &ZAIProvider,
                session_id: uuid::Uuid,
                preamble: Option<&str>,
                thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

    let _codex = |p: &CodexProvider,
                  session_id: uuid::Uuid,
                  preamble: Option<&str>,
                  thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

    let _copilot = |p: &CopilotProvider,
                    session_id: uuid::Uuid,
                    preamble: Option<&str>,
                    thinking: Option<serde_json::Value>| {
        p.create_rig_agent(session_id, preamble, thinking)
    };

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
