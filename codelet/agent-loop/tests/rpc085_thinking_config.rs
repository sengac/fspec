//! Feature: spec/features/agent-loop-thinking-config.feature
//!
//! RPC-085 (RPC-072 family): thinking config wiring parity. The canonical
//! NAPI agent loop computes an effective thinking config per turn and
//! threads it as the 3rd positional argument of every provider's
//! `create_rig_agent` invocation. The three dispatch paths are:
//!
//!   1. The `run_with_provider!` macro in `dispatch.rs` (used by the
//!      `claude`, `gemini`, `zai`, `codex`, and `github-copilot` /
//!      `copilot` arms).
//!   2. The inlined `"openai" => { ... }` match arm in `agent_loop.rs`
//!      (inlined because `get_openai` requires `session.id`).
//!   3. The `_ =>` custom-provider fallthrough in `agent_loop.rs` which
//!      calls the free function `codelet_providers::custom::
//!      CustomProvider::create_rig_agent`.
//!
//! This file pins the contract via structural source-string assertions
//! and compile-time closure signatures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers — locate source files relative to CARGO_MANIFEST_DIR.
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
/// `dispatch.rs`. Returns the substring spanning the outer `{ ... }`.
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
/// `agent_loop.rs`.
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

/// Extract the custom-provider `_ => { ... }` fallthrough arm body
/// surrounding `CustomProvider::create_rig_agent`.
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

/// Count occurrences of a literal substring.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut n = 0usize;
    let mut start = 0usize;
    while let Some(pos) = haystack[start..].find(needle) {
        n += 1;
        start += pos + needle.len();
    }
    n
}

/// Parse the positional arguments of a function-call expression starting
/// at `call_start_offset` in `src`. Returns the trimmed arg strings.
fn parse_positional_args(src: &str, call_start_offset: usize) -> Vec<String> {
    let tail = &src[call_start_offset..];
    let open_paren = tail.find('(').expect("call site must include `(`");
    let bytes = tail.as_bytes();
    let mut depth: i32 = 0;
    let mut starts: Vec<usize> = vec![open_paren + 1];
    let mut commas: Vec<usize> = Vec::new();
    let mut i = open_paren;
    let mut close = usize::MAX;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = i;
                    break;
                }
            }
            b',' if depth == 1 => {
                commas.push(i);
                starts.push(i + 1);
            }
            _ => {}
        }
        i += 1;
    }
    assert_ne!(close, usize::MAX, "unterminated call:\n{tail}");
    let mut ends: Vec<usize> = commas.clone();
    ends.push(close);
    starts
        .iter()
        .zip(ends.iter())
        .map(|(s, e)| tail[*s..*e].trim().to_string())
        .filter(|seg| !seg.is_empty())
        .collect()
}

// ===========================================================================
// Scenario: InputWithImages carries thinking_config alongside text and images
// ===========================================================================

#[test]
fn input_with_images_declares_thinking_config_option_string_field() {
    // @step Given the dispatch helper struct in codelet/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When the structural source is inspected
    // @step Then InputWithImages declares a thinking_config field of type Option<String>
    assert!(
        src.contains("pub(crate) struct InputWithImages"),
        "dispatch.rs must declare `pub(crate) struct InputWithImages`"
    );
    let struct_pos = src
        .find("pub(crate) struct InputWithImages")
        .expect("struct InputWithImages must be present");
    let struct_tail = &src[struct_pos..];
    let body_open = struct_tail
        .find('{')
        .expect("struct body must start with `{`");
    let bytes = struct_tail.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_open;
    let mut body_end = usize::MAX;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    assert_ne!(
        body_end,
        usize::MAX,
        "InputWithImages struct body must close"
    );
    let body = &struct_tail[body_open..=body_end];
    assert!(
        body.contains("pub(crate) thinking_config: Option<String>"),
        "InputWithImages must declare `pub(crate) thinking_config: Option<String>`. body:\n{body}"
    );

    // @step And the field documents that it is a per-turn override superimposed on session_thinking_level
    // The doc comment "session_thinking_level" appears in the field's doc-block.
    let field_pos = body
        .find("thinking_config: Option<String>")
        .expect("field declaration must be present");
    let prelude = &body[..field_pos];
    assert!(
        prelude.contains("session_thinking_level"),
        "thinking_config field doc-block must reference `session_thinking_level`. prelude:\n{prelude}"
    );
}

// ===========================================================================
// Scenario: run_with_provider! macro forwards thinking config to create_rig_agent
// ===========================================================================

#[test]
fn run_with_provider_macro_threads_thinking_into_create_rig_agent() {
    // @step Given the run_with_provider! macro_rules! body in codelet/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When the macro body is parsed
    let body = extract_run_with_provider_macro_body(&src);

    // @step Then the macro accepts a $thinking metavariable as its 7th positional argument
    // The macro arm header lives in src, preceding the body. Locate it.
    let arm_header_start = src
        .find("($inner:expr,")
        .expect("macro arm header must start with `($inner:expr,`");
    let arm_header_tail = &src[arm_header_start..];
    let arrow = arm_header_tail
        .find(") => {")
        .expect("macro arm header must close with `) => {`");
    let header = &arm_header_tail[..arrow + 1];
    // Split on top-level commas — depth here is irrelevant because the arm header is flat.
    let metavars: Vec<&str> = header
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .map(|s| s.trim())
        .collect();
    assert_eq!(
        metavars.len(),
        7,
        "run_with_provider! arm header must accept 7 metavariables; got {}. header:\n{header}",
        metavars.len()
    );
    assert_eq!(
        metavars[6], "$thinking:expr",
        "7th metavariable must be `$thinking:expr`; got `{}`",
        metavars[6]
    );

    // @step And the macro invokes provider.create_rig_agent with role_preamble.as_deref() and $thinking.clone() as the 2nd and 3rd positional arguments
    let create_call = body
        .find("provider.create_rig_agent")
        .expect("macro body must call `provider.create_rig_agent`");
    let args = parse_positional_args(body, create_call);
    assert_eq!(
        args.len(),
        3,
        "provider.create_rig_agent in macro body must take 3 positional args; got {}. args: {args:?}",
        args.len()
    );
    assert_eq!(args[0], "$session.id");
    assert_eq!(args[1], "role_preamble.as_deref()");
    assert_eq!(args[2], "$thinking.clone()");
}

// ===========================================================================
// Scenario: agent_loop body computes thinking_config_value per turn
// ===========================================================================

#[test]
fn agent_loop_body_computes_thinking_config_value_per_turn() {
    // @step Given the agent loop body in codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When the source is scanned
    // @step Then a thinking_config_value binding of type Option<serde_json::Value> is computed once per turn
    assert!(
        src.contains("let thinking_config_value: Option<serde_json::Value>"),
        "agent_loop.rs must compute `let thinking_config_value: Option<serde_json::Value>`"
    );

    // @step And the computation references compute_effective_thinking_level, is_adaptive_thinking_model, and get_thinking_config
    let binding_pos = src
        .find("let thinking_config_value: Option<serde_json::Value>")
        .expect("thinking_config_value binding must exist");
    // Scope: the block scope opens at the next `{` and closes when depth returns to 0.
    let tail = &src[binding_pos..];
    let block_open = tail.find('{').expect("binding block must start with `{`");
    let bytes = tail.as_bytes();
    let mut depth: i32 = 0;
    let mut i = block_open;
    let mut block_end = usize::MAX;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    block_end = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    assert_ne!(
        block_end,
        usize::MAX,
        "thinking_config_value block must close"
    );
    let block = &tail[block_open..=block_end];

    for needle in [
        "compute_effective_thinking_level",
        "is_adaptive_thinking_model",
        "get_thinking_config",
    ] {
        assert!(
            block.contains(needle),
            "thinking_config_value computation block must reference `{needle}`. block:\n{block}"
        );
    }

    // @step And the computation honours the PROV-005 priority order (adaptive first, then TS-passed config, then unified detection)
    // The branch order is: `if is_adaptive_model { ... } else if let Some(config_str) = input_with_images.thinking_config.as_deref() { ... } else { ... }`
    let adaptive_pos = block
        .find("if is_adaptive_model")
        .expect("adaptive-first branch must come first");
    let ts_pos = block
        .find("else if let Some(config_str) = input_with_images.thinking_config.as_deref()")
        .expect("TS-passed config branch must follow adaptive");
    let unified_pos = block
        .rfind("} else {")
        .expect("unified detection else branch must exist");
    assert!(
        adaptive_pos < ts_pos && ts_pos < unified_pos,
        "PROV-005 priority order must be adaptive → TS-passed → unified detection. \
         positions: adaptive={adaptive_pos}, ts={ts_pos}, unified={unified_pos}"
    );
}

// ===========================================================================
// Scenario: All run_with_provider! call sites pass thinking_config_value
// ===========================================================================

#[test]
fn all_run_with_provider_invocations_pass_thinking_config_value() {
    // @step Given the agent loop body in codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When every invocation of the run_with_provider! macro is enumerated
    // Locate all `run_with_provider!` invocations and parse their args.
    let mut invocation_starts: Vec<usize> = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = src[start..].find("run_with_provider!(") {
        invocation_starts.push(start + pos);
        start += pos + "run_with_provider!(".len();
    }

    assert!(
        invocation_starts.len() >= 5,
        "must have ≥5 run_with_provider! invocations (claude, gemini, zai, codex, github-copilot/copilot); \
         found {}",
        invocation_starts.len()
    );

    // @step Then each invocation passes thinking_config_value as its 7th positional argument
    for inv_start in &invocation_starts {
        let args = parse_positional_args(&src, *inv_start);
        assert_eq!(
            args.len(),
            7,
            "run_with_provider! at byte {inv_start} must have 7 positional args; got {}. args: {args:?}",
            args.len()
        );
        assert_eq!(
            args[6], "thinking_config_value",
            "7th positional arg of run_with_provider! at byte {inv_start} must be `thinking_config_value`; got `{}`",
            args[6]
        );
    }

    // @step And the enumerated providers cover claude, gemini, zai, codex, and copilot
    // The match arm pattern preceding each invocation identifies the provider.
    let expected_providers = [
        "\"claude\"",
        "\"gemini\"",
        "\"zai\"",
        "\"codex\"",
        "\"github-copilot\" | \"copilot\"",
    ];
    for expected in expected_providers {
        let arrow_marker = format!("{expected} => run_with_provider!(");
        let multiline_marker = format!("{expected} => run_with_provider!(\n");
        assert!(
            src.contains(&arrow_marker) || src.contains(&multiline_marker),
            "agent_loop.rs must have an arm `{expected} => run_with_provider!(...)`"
        );
    }
}

// ===========================================================================
// Scenario: OpenAI inlined arm passes thinking_config_value to create_rig_agent
// ===========================================================================

#[test]
fn openai_inlined_arm_passes_thinking_config_value_to_create_rig_agent() {
    // @step Given the inlined "openai" => { ... } match arm in codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When the arm body is parsed
    let arm = extract_openai_match_arm(&src);

    // @step Then provider.create_rig_agent is invoked with session.id, role_preamble.as_deref(), and thinking_config_value.clone() as the 1st, 2nd, and 3rd positional arguments
    let create_call = arm
        .find("provider.create_rig_agent")
        .expect("OpenAI arm must call provider.create_rig_agent");
    let args = parse_positional_args(arm, create_call);
    assert_eq!(
        args.len(),
        3,
        "OpenAI provider.create_rig_agent must take 3 positional args; got {}. args: {args:?}",
        args.len()
    );
    assert_eq!(args[0], "session.id");
    assert_eq!(args[1], "role_preamble.as_deref()");
    assert_eq!(args[2], "thinking_config_value.clone()");
    // Sanity: the arm must contain exactly one create_rig_agent call.
    assert_eq!(
        count_occurrences(arm, "provider.create_rig_agent"),
        1,
        "OpenAI inlined arm must invoke provider.create_rig_agent exactly once"
    );
}

// ===========================================================================
// Scenario: Custom-provider fallthrough passes thinking_config_value to create_rig_agent
// ===========================================================================

#[test]
fn custom_provider_fallthrough_passes_thinking_config_value() {
    // @step Given the `_ =>` custom-provider fallthrough match arm in codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When the arm body is parsed
    let arm = extract_custom_provider_fallthrough_arm(&src);

    // @step Then codelet_providers::custom::CustomProvider::create_rig_agent is invoked
    let qualified_call = arm
        .find("codelet_providers::custom::CustomProvider::create_rig_agent(")
        .expect("custom-provider fallthrough must invoke the fully-qualified path");

    // @step And the final three positional arguments are session.id, role_preamble.as_deref(), and thinking_config_value.clone()
    let args = parse_positional_args(arm, qualified_call);
    assert!(
        args.len() >= 3,
        "CustomProvider::create_rig_agent must have ≥3 positional args; got {}. args: {args:?}",
        args.len()
    );
    let last_three = &args[args.len() - 3..];
    assert_eq!(last_three[0], "session.id");
    assert_eq!(last_three[1], "role_preamble.as_deref()");
    assert_eq!(last_three[2], "thinking_config_value.clone()");
}

// ===========================================================================
// Scenario: Provider create_rig_agent signature compiles for all 6
//           instance-based built-in providers
// ===========================================================================

#[test]
fn provider_create_rig_agent_signature_compiles_for_six_instance_providers() {
    // @step Given the create_rig_agent method on each of Claude, OpenAI, Gemini, ZAI, Codex, and Copilot
    use codelet_providers::claude::ClaudeProvider;
    use codelet_providers::codex::CodexProvider;
    use codelet_providers::copilot::CopilotProvider;
    use codelet_providers::gemini::GeminiProvider;
    use codelet_providers::openai::OpenAIProvider;
    use codelet_providers::zai::ZAIProvider;

    // @step When a no-op closure pins the signature (uuid::Uuid, Option<&str>, Option<serde_json::Value>) -> RigAgentHandle
    let _claude_sig = |p: &ClaudeProvider,
                       id: uuid::Uuid,
                       preamble: Option<&str>,
                       thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };
    let _openai_sig = |p: &OpenAIProvider,
                       id: uuid::Uuid,
                       preamble: Option<&str>,
                       thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };
    let _gemini_sig = |p: &GeminiProvider,
                       id: uuid::Uuid,
                       preamble: Option<&str>,
                       thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };
    let _zai_sig = |p: &ZAIProvider,
                    id: uuid::Uuid,
                    preamble: Option<&str>,
                    thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };
    let _codex_sig = |p: &CodexProvider,
                      id: uuid::Uuid,
                      preamble: Option<&str>,
                      thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };
    let _copilot_sig = |p: &CopilotProvider,
                        id: uuid::Uuid,
                        preamble: Option<&str>,
                        thinking: Option<serde_json::Value>| {
        p.create_rig_agent(id, preamble, thinking)
    };

    // @step Then the closure compiles for every provider type without coercion
    // (Compile-time success is the assertion. A runtime check confirms the
    // closures are not dead-code-eliminated.)
    let _ = (
        &_claude_sig as &dyn std::any::Any,
        &_openai_sig as &dyn std::any::Any,
        &_gemini_sig as &dyn std::any::Any,
        &_zai_sig as &dyn std::any::Any,
        &_codex_sig as &dyn std::any::Any,
        &_copilot_sig as &dyn std::any::Any,
    );
}

// ===========================================================================
// Scenario: CustomProvider create_rig_agent signature accepts thinking_config
//           as its last argument
// ===========================================================================

#[test]
fn custom_provider_create_rig_agent_signature_compiles() {
    use codelet_providers::custom::CustomProvider;

    // @step Given the free function codelet_providers::custom::CustomProvider::create_rig_agent
    // @step When a no-op closure pins the signature (&Path, &str, &str, uuid::Uuid, Option<&str>, Option<serde_json::Value>)
    let _custom_sig = |project_root: &std::path::Path,
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

    // @step Then the closure compiles without coercion
    // @step And thinking_config is the 6th positional argument
    let _ = &_custom_sig as &dyn std::any::Any;
}
