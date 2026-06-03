//! Feature: spec/features/agent-loop-role-injection.feature
//!
//! RPC-082 (BUG-120 parity): the canonical NAPI agent loop
//! (`codelet/napi/src/agent_loop.rs:91-96`) reads `session.get_role()`
//! every turn and passes the result as the `preamble` argument to
//! `provider.create_rig_agent`. After the RPC-080/RPC-081 port the
//! same plumbing now lives in the NAPI-free `codelet-agent-loop` crate.
//! This test file pins the BUG-120 contract via:
//!
//!   1. A behavioural round-trip on `BackgroundSession::{set_role,
//!      get_role, clear_role}` constructed through a real
//!      [`codelet_sessions::SessionManager`].
//!   2. String-based structural assertions over the macro body in
//!      `dispatch.rs` and the inlined match arms in `agent_loop.rs`.
//!   3. Compile-time closure assertions proving every provider's
//!      `create_rig_agent` signature accepts a `preamble: Option<&str>`
//!      in the second positional slot (fifth for `CustomProvider`).
//!
//! Scenarios covered (1 feature = 1 test file):
//!   - "BackgroundSession round-trips set_role / get_role / clear_role"
//!   - "run_with_provider! macro reads session.get_role() and passes it
//!     to create_rig_agent"
//!   - "OpenAI inlined arm reads session.get_role() and passes it to
//!     create_rig_agent"
//!   - "Custom-provider fallthrough arm reads session.get_role() and
//!     passes it to CustomProvider::create_rig_agent"
//!   - "Every dispatched provider type accepts an Option<&str> preamble
//!     in create_rig_agent"

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
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
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
    // Walk braces to find the matching close of the arm body.
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
    // Walk backwards from the call to find the enclosing `_ => {`.
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
    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());

    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_id_str = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&session_id_str, "stub/canned", &project, "role-test-session")
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
// Scenario: run_with_provider! macro reads session.get_role() and passes
//           it to create_rig_agent
// ===========================================================================

#[test]
fn run_with_provider_macro_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file codelet/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When the macro body of run_with_provider! is extracted
    let body = extract_run_with_provider_macro_body(&src);

    // @step Then it contains the expression "session.get_role()"
    //
    // Macro meta-variables use `$session` inside the macro definition,
    // so the canonical token in dispatch.rs is `$session.get_role()`.
    assert!(
        body.contains("$session.get_role()"),
        "run_with_provider! macro body must read the session role via \
         `$session.get_role()`; body was:\n{body}"
    );

    // @step And it binds the result to a "role_preamble" local
    assert!(
        body.contains("let role_preamble = $session.get_role();"),
        "macro body must bind the role to a `role_preamble` local; body was:\n{body}"
    );

    // @step And it passes "role_preamble.as_deref()" as the second
    //       positional argument to "provider.create_rig_agent"
    assert!(
        body.contains("provider.create_rig_agent("),
        "macro body must invoke `provider.create_rig_agent(`; body was:\n{body}"
    );
    assert!(
        body.contains("role_preamble.as_deref()"),
        "macro body must pass `role_preamble.as_deref()` into create_rig_agent; \
         body was:\n{body}"
    );

    // Pin positional order: session.id first, role_preamble.as_deref() second.
    let call_start = body
        .find("provider.create_rig_agent(")
        .expect("create_rig_agent call site must exist (checked above)");
    let call_tail = &body[call_start..];
    let session_pos = call_tail
        .find("$session.id")
        .expect("first positional arg must be `$session.id`");
    let preamble_pos = call_tail
        .find("role_preamble.as_deref()")
        .expect("`role_preamble.as_deref()` must appear inside create_rig_agent");
    assert!(
        session_pos < preamble_pos,
        "`$session.id` must precede `role_preamble.as_deref()` as the second \
         positional argument; call tail:\n{call_tail}"
    );
}

// ===========================================================================
// Scenario: OpenAI inlined arm reads session.get_role() and passes it to
//           create_rig_agent
// ===========================================================================

#[test]
fn openai_inlined_arm_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When the inlined "openai" match arm is extracted
    let body = extract_openai_match_arm(&src);

    // @step Then it contains "let role_preamble = session.get_role();"
    assert!(
        body.contains("let role_preamble = session.get_role();"),
        "openai arm must bind `role_preamble` to `session.get_role()`; body was:\n{body}"
    );

    // @step And the subsequent provider.create_rig_agent call uses
    //       "role_preamble.as_deref()" as the second argument
    let call_start = body
        .find("provider.create_rig_agent(")
        .expect("openai arm must call `provider.create_rig_agent(`");
    let call_tail = &body[call_start..];
    let session_pos = call_tail
        .find("session.id")
        .expect("first arg to create_rig_agent must be `session.id`");
    let preamble_pos = call_tail
        .find("role_preamble.as_deref()")
        .expect("openai arm must pass `role_preamble.as_deref()` as preamble");
    assert!(
        session_pos < preamble_pos,
        "`session.id` must come before `role_preamble.as_deref()` in the create_rig_agent \
         call; call tail:\n{call_tail}"
    );
}

// ===========================================================================
// Scenario: Custom-provider fallthrough arm reads session.get_role() and
//           passes it to CustomProvider::create_rig_agent
// ===========================================================================

#[test]
fn custom_provider_fallthrough_reads_session_role_and_passes_as_preamble() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When the "_" fallthrough match arm is extracted
    let body = extract_custom_provider_fallthrough_arm(&src);

    // @step Then it contains "let role_preamble = session.get_role();"
    assert!(
        body.contains("let role_preamble = session.get_role();"),
        "custom-provider arm must bind `role_preamble` to `session.get_role()`; \
         body was:\n{body}"
    );

    // @step And the subsequent CustomProvider::create_rig_agent call uses
    //       "role_preamble.as_deref()" as the fifth positional argument
    let call_start = body
        .find("CustomProvider::create_rig_agent(")
        .expect("custom-provider arm must call `CustomProvider::create_rig_agent(`");
    // Walk the parenthesised positional args to confirm fifth-slot identity.
    // Signature is:
    //   CustomProvider::create_rig_agent(
    //       &project_root,         // 1
    //       &current_provider,     // 2
    //       &model_alias,          // 3
    //       session.id,            // 4
    //       role_preamble.as_deref(), // 5
    //       thinking_config_value.clone(), // 6
    //   )
    let args_start = call_start + "CustomProvider::create_rig_agent(".len();
    let bytes = body.as_bytes();
    let mut depth: i32 = 1; // we already consumed the opening `(`
    let mut i = args_start;
    let args_end;
    loop {
        assert!(
            i < bytes.len(),
            "unterminated CustomProvider::create_rig_agent call site"
        );
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    args_end = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    // Split the argument region on top-level commas only — i.e. commas
    // whose paren-depth (relative to the args region) is zero.
    let args_region = &body[args_start..args_end];
    let mut positional: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut arg_depth: i32 = 0;
    for ch in args_region.chars() {
        match ch {
            '(' => {
                arg_depth += 1;
                current.push(ch);
            }
            ')' => {
                arg_depth -= 1;
                current.push(ch);
            }
            ',' if arg_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    positional.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed_last = current.trim();
    if !trimmed_last.is_empty() {
        positional.push(trimmed_last.to_string());
    }
    assert!(
        positional.len() >= 5,
        "CustomProvider::create_rig_agent must have at least 5 positional args; \
         found {} in:\n{args_region}",
        positional.len()
    );
    assert_eq!(
        positional[4], "role_preamble.as_deref()",
        "fifth positional argument must be `role_preamble.as_deref()`; \
         got `{}`",
        positional[4]
    );
}

// ===========================================================================
// Scenario: Every dispatched provider type accepts an Option<&str>
//           preamble in create_rig_agent
// ===========================================================================
//
// Pattern adapted from
// `codelet/agent-loop/src/dispatch.rs::copilot_create_rig_agent_signature_matches_dispatch_macro_contract`.
// Each closure is a never-called type witness — its body provides the
// proof that the provider exposes the
//   create_rig_agent(session_id, preamble: Option<&str>, thinking)
// signature the dispatch macro depends on.

#[test]
fn every_provider_create_rig_agent_signature_accepts_option_str_preamble() {
    use codelet_providers::claude::ClaudeProvider;
    use codelet_providers::codex::CodexProvider;
    use codelet_providers::copilot::CopilotProvider;
    use codelet_providers::custom::custom_provider::CustomProvider;
    use codelet_providers::gemini::GeminiProvider;
    use codelet_providers::openai::OpenAIProvider;
    use codelet_providers::zai::ZAIProvider;

    // @step Given the seven dispatched provider arms (claude, openai,
    //       gemini, zai, codex, github-copilot, copilot) plus the
    //       custom-provider fallthrough

    // @step Then each provider type exposes
    //       create_rig_agent(session_id, preamble: Option<&str>, thinking)
    // @step And a compile-time closure assertion against the signature
    //       succeeds for each provider

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

    // CustomProvider has a static `create_rig_agent` — the signature is:
    //   fn create_rig_agent(
    //       project_root: &Path,
    //       name: &str,
    //       model_alias: &str,
    //       session_id: uuid::Uuid,
    //       preamble: Option<&str>,
    //       thinking_config: Option<serde_json::Value>,
    //   ) -> Result<CustomRigAgent, CustomProviderError>
    let _custom = |project_root: &std::path::Path,
                   name: &str,
                   model_alias: &str,
                   session_id: uuid::Uuid,
                   preamble: Option<&str>,
                   thinking: Option<serde_json::Value>| {
        CustomProvider::create_rig_agent(
            project_root,
            name,
            model_alias,
            session_id,
            preamble,
            thinking,
        )
    };

    // Structural assertion: the dispatch predicate still covers the
    // seven macro-driven provider names, so the dispatch macro can
    // legally substitute every closure above.
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
             signature contract above is reachable through the agent loop"
        );
    }
}
