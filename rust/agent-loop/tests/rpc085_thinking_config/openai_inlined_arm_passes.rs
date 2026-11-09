//! Feature: spec/features/agent-loop-thinking-config.feature
use super::*;

#[test]
fn openai_inlined_arm_passes_thinking_config_value_to_create_rig_agent() {
    // @step Given the run_openai_turn function in rust/agent-loop/src/agent_loop/provider_openai.rs
    let src = read_source("agent_loop/provider_openai.rs");

    // @step When the arm body is parsed
    let body = &src[src
        .find("fn run_openai_turn(")
        .expect("provider_openai.rs must define run_openai_turn")..];

    // @step Then provider.create_rig_agent is invoked with session.id, role_preamble.as_deref(), and thinking_config_value as the 1st, 2nd, and 3rd positional arguments
    let create_call = body
        .find("provider.create_rig_agent")
        .expect("run_openai_turn must call provider.create_rig_agent");
    let args = parse_positional_args(body, create_call);
    assert_eq!(
        args,
        [
            "session.id",
            "role_preamble.as_deref()",
            "thinking_config_value"
        ]
    );
    assert_eq!(
        count_occurrences(body, "provider.create_rig_agent"),
        1,
        "run_openai_turn must invoke provider.create_rig_agent exactly once"
    );
}

#[test]
fn custom_provider_fallthrough_passes_thinking_config_value() {
    // @step Given the run_custom_turn custom-provider path in rust/agent-loop/src/agent_loop/provider_custom.rs
    let src = read_source("agent_loop/provider_custom.rs");

    // @step When the arm body is parsed
    let body = &src[src
        .find("fn run_custom_turn(")
        .expect("provider_custom.rs must define run_custom_turn")..];

    // @step Then codelet_providers::custom::CustomProvider::create_rig_agent is invoked
    let qualified_call = body
        .find("codelet_providers::custom::CustomProvider::create_rig_agent(")
        .expect("run_custom_turn must invoke the fully-qualified path");

    // @step And the final four positional arguments are session.id, role_preamble.as_deref(), thinking_config_value and the sub-agent flag false
    let args = parse_positional_args(body, qualified_call);
    assert!(
        args.len() >= 4,
        "CustomProvider::create_rig_agent must have ≥4 positional args; got {}. args: {args:?}",
        args.len()
    );
    // CMPCT-044/045: the parent agent loop keeps the full custom-provider
    // surface (the compactor spawner is the only call site passing `true`).
    assert_eq!(
        &args[args.len() - 4..],
        [
            "session.id",
            "role_preamble.as_deref()",
            "thinking_config_value",
            "false"
        ]
    );
}

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
                       thinking: Option<serde_json::Value>,
                       sub_agent: bool| {
        CustomProvider::create_rig_agent(
            project_root,
            name,
            model_alias,
            session_id,
            preamble,
            thinking,
            sub_agent,
        )
    };

    // @step Then the closure compiles without coercion
    // @step And thinking_config is the 6th positional argument
    let _ = &_custom_sig as &dyn std::any::Any;
}
