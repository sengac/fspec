//! Feature: spec/features/agent-loop-thinking-config.feature
use super::*;

#[test]
fn input_with_images_declares_thinking_config_option_string_field() {
    // @step Given the dispatch helper struct in rust/agent-loop/src/dispatch.rs
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

#[test]
fn run_with_provider_macro_threads_thinking_into_create_rig_agent() {
    // @step Given the run_with_provider! macro_rules! body in rust/agent-loop/src/dispatch.rs
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

#[test]
fn agent_loop_body_computes_thinking_config_value_per_turn() {
    // @step Given the agent loop body in rust/agent-loop/src/agent_loop.rs and its thinking module
    let src = read_source("agent_loop.rs");
    let thinking = read_source("agent_loop/thinking.rs");

    // @step When the source is scanned
    // @step Then a thinking_config_value of type Result<Option<serde_json::Value>, String> is computed once per turn by resolve_turn_thinking
    assert_eq!(
        count_occurrences(
            &src,
            // BUG-439: the turn is refused when the level cannot be expressed.
            "let thinking_config_value = match thinking::resolve_turn_thinking("
        ),
        1,
        "agent_loop.rs must bind thinking_config_value from resolve_turn_thinking once per turn"
    );
    let fn_pos = thinking
        .find("fn resolve_turn_thinking(")
        .expect("agent_loop/thinking.rs must define resolve_turn_thinking");
    let tail = &thinking[fn_pos..];
    let returns = tail
        .find('{')
        .expect("resolve_turn_thinking must have a body");
    assert!(
        tail[..returns].contains("-> Result<Option<serde_json::Value>, String>"),
        "resolve_turn_thinking must return Result<Option<serde_json::Value>, String> (BUG-439)"
    );

    // @step And the computation reads the session's level with base_thinking_level and builds the wire form through the turn's ThinkingTarget
    let bytes = tail.as_bytes();
    let mut depth: i32 = 0;
    let mut block_end = usize::MAX;
    for (offset, byte) in bytes.iter().enumerate().skip(returns) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    block_end = offset;
                    break;
                }
            }
            _ => {}
        }
    }
    assert_ne!(
        block_end,
        usize::MAX,
        "resolve_turn_thinking body must close"
    );
    let block = &tail[returns..=block_end];
    // MODEL-010/MODEL-011: the session's level (None = model default) is decoded once
    // and the turn's ThinkingTarget builds its provider's wire form.
    for needle in ["base_thinking_level(", "target.config_for("] {
        assert!(
            block.contains(needle),
            "resolve_turn_thinking must reference `{needle}`. block:\n{block}"
        );
    }

    // @step And a per-turn raw thinking config is refused before the session's level is used
    let refusal_pos = block
        .find("if thinking_config.is_some()")
        .expect("the per-turn raw thinking config must be refused");
    let level_pos = block
        .find("base_thinking_level(")
        .expect("the session level must be read");
    assert!(
        refusal_pos < level_pos,
        "the per-turn raw config is refused before the session level is used. \
         positions: refusal={refusal_pos}, level={level_pos}"
    );
}

#[test]
fn all_run_with_provider_invocations_pass_thinking_config_value() {
    // @step Given the agent loop body in rust/agent-loop/src/agent_loop.rs and its provider modules
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
