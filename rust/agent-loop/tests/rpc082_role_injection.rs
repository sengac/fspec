//! Feature: spec/features/agent-loop-role-injection.feature
//!
//! RPC-082 (BUG-120 parity): every agent turn reads `session.get_role()` and
//! passes it as the `preamble` argument of `create_rig_agent`. Pinned by a
//! behavioural round-trip of `BackgroundSession::{set_role, get_role,
//! clear_role}`, structural checks of the code that builds each provider's
//! agent (BUG-344: the whole `run_with_provider!` macro in dispatch.rs,
//! `run_openai_turn` in agent_loop/provider_openai.rs and `run_custom_turn` in
//! agent_loop/provider_custom.rs) and compile-time signature witnesses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "provider_sources/support.rs"]
#[macro_use]
mod support;

use support::{assert_in_order, function_body, macro_body, read_source};

// ===========================================================================
// Scenario: BackgroundSession round-trips set_role / get_role / clear_role
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_session_round_trips_set_get_clear_role() {
    use std::sync::Arc;

    use codelet_agent_loop::FspecAgentHooks;
    use codelet_sessions::session_manager::SessionManager;
    use uuid::Uuid;

    // Set up the same hermetic environment used by the RPC-072 round-trip.
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");

    let manager = SessionManager::new();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    assert_eq!(manager.set_default_model("stub/canned"), Ok(()));

    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_id_str = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &session_id_str,
            "stub/canned",
            &project,
            "role-test-session",
        )
        .await
        .expect("create_session_with_id");

    // @step Given a fresh BackgroundSession
    let session = manager
        .get_session(&session_id_str)
        .expect("session must exist after create_session_with_id");

    // @step When get_role is called
    let role_initial = session.get_role();

    // @step Then it returns None
    assert!(
        role_initial.is_none(),
        "fresh BackgroundSession must report no role; got {role_initial:?}"
    );

    // @step When set_role is called with "You are a pirate"
    session.set_role("You are a pirate".to_string());

    // @step And get_role is called
    let role_after_set = session.get_role();

    // @step Then it returns Some("You are a pirate")
    assert_eq!(
        role_after_set.as_deref(),
        Some("You are a pirate"),
        "set_role must round-trip through get_role verbatim"
    );

    // @step When clear_role is called
    session.clear_role();

    // @step And get_role is called
    let role_after_clear = session.get_role();

    // @step Then it returns None
    assert!(
        role_after_clear.is_none(),
        "clear_role must reset get_role to None; got {role_after_clear:?}"
    );
}

// ===========================================================================
// Scenario: run_with_provider! macro reads session.get_role() and passes it
//           to create_rig_agent
// ===========================================================================

#[test]
fn run_with_provider_macro_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file rust/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When the macro body of run_with_provider! is extracted with all of its arms
    let body = macro_body(&src, "run_with_provider");

    // @step Then it contains the expression "session.get_role()"
    assert!(body.contains("$session.get_role()"), "{body}");

    // @step And it binds the result to a "role_preamble" local
    assert!(
        body.contains("let role_preamble = $session.get_role();"),
        "{body}"
    );

    // @step And it passes "role_preamble.as_deref()" as the second positional argument to "provider.create_rig_agent"
    assert_in_order(
        body,
        &[
            "provider.create_rig_agent(",
            "$session.id,",
            "role_preamble.as_deref(),",
        ],
    );
}

// ===========================================================================
// Scenario: OpenAI turn reads session.get_role() and passes it to create_rig_agent
// ===========================================================================

#[test]
fn openai_turn_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file rust/agent-loop/src/agent_loop/provider_openai.rs
    let src = read_source("agent_loop/provider_openai.rs");

    // @step When the run_openai_turn function body is extracted
    let body = function_body(&src, "run_openai_turn");

    // @step Then it contains "let role_preamble = session.get_role();"
    assert!(
        body.contains("let role_preamble = session.get_role();"),
        "{body}"
    );

    // @step And the subsequent provider.create_rig_agent call uses "role_preamble.as_deref()" as the second argument
    assert_in_order(
        body,
        &[
            "let role_preamble = session.get_role();",
            "provider.create_rig_agent(",
            "session.id,",
            "role_preamble.as_deref(),",
        ],
    );
}

// ===========================================================================
// Scenario: Custom-provider turn reads session.get_role() and passes it to
//           CustomProvider::create_rig_agent
// ===========================================================================

#[test]
fn custom_provider_turn_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file rust/agent-loop/src/agent_loop/provider_custom.rs
    let src = read_source("agent_loop/provider_custom.rs");

    // @step When the run_custom_turn function body is extracted
    let body = function_body(&src, "run_custom_turn");

    // @step Then it contains "let role_preamble = session.get_role();"
    assert!(
        body.contains("let role_preamble = session.get_role();"),
        "{body}"
    );

    // @step And the subsequent CustomProvider::create_rig_agent call uses "role_preamble.as_deref()" as the fifth positional argument
    let call = &body[body
        .find("CustomProvider::create_rig_agent(")
        .expect("run_custom_turn must call CustomProvider::create_rig_agent")..];
    let args: Vec<&str> = call["CustomProvider::create_rig_agent(".len()..]
        .split(',')
        .map(str::trim)
        .collect();
    assert_eq!(
        args.get(4).copied(),
        Some("role_preamble.as_deref()"),
        "{call}"
    );
}

// ===========================================================================
// Scenario: Every dispatched provider type accepts an Option<&str> preamble
//           in create_rig_agent
// ===========================================================================

#[test]
fn every_provider_create_rig_agent_signature_accepts_option_str_preamble() {
    use codelet_providers::custom::custom_provider::CustomProvider;

    // @step Given the seven dispatched provider arms (claude, openai, gemini, zai, codex, github-copilot, copilot) plus the custom-provider fallthrough
    let dispatched = [
        "claude",
        "openai",
        "gemini",
        "zai",
        "codex",
        "github-copilot",
        "copilot",
    ];

    // @step Then each provider type exposes create_rig_agent(session_id, preamble: Option<&str>, thinking)
    preamble_signature_witness!(
        codelet_providers::claude::ClaudeProvider,
        codelet_providers::openai::OpenAIProvider,
        codelet_providers::gemini::GeminiProvider,
        codelet_providers::zai::ZAIProvider,
        codelet_providers::codex::CodexProvider,
        codelet_providers::copilot::CopilotProvider
    );

    // @step And a compile-time closure assertion against the signature succeeds for each provider
    let _custom = |root: &std::path::Path,
                   name: &str,
                   alias: &str,
                   session_id: uuid::Uuid,
                   preamble: Option<&str>,
                   thinking: Option<serde_json::Value>,
                   sub_agent: bool| {
        CustomProvider::create_rig_agent(
            root, name, alias, session_id, preamble, thinking, sub_agent,
        )
    };
    for provider in dispatched {
        assert!(
            codelet_agent_loop::agent_loop_dispatch_supports_provider(provider),
            "dispatch predicate must list `{provider}`"
        );
    }
}
