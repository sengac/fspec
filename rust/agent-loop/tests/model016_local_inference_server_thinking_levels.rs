//! MODEL-016 — local inference servers declare their thinking levels,
//! including numeric efforts, and get them in the form their engine takes.
//!
//! Feature: spec/features/local-inference-server-thinking-levels.feature

mod model016_support;

use codelet_agent_loop::thinking_config::TurnLevel;
use codelet_providers::local_reasoning::LiveCheckReport;
use codelet_providers::models::ReasoningLevel;
use codelet_providers::OpenAIProvider;
use codelet_test_helpers::{OrFail, TestResult};
use model016_support::*;
use rig::completion::Completion;
use serde_json::{json, Number, Value};
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const QWEN_NEXT: &str = "qwen3.8-flash-next";

fn strata_qwen_next() -> Value {
    one_model(
        "strata",
        QWEN_NEXT,
        Some(efforts(&["none", "low", "medium", "xhigh"])),
    )
}

#[test]
#[serial]
fn a_declared_effort_is_sent_to_strata_unchanged() -> TestResult {
    // @step Given a Strata profile whose model "qwen3.8-flash-next" declares efforts "none, low, medium, xhigh"
    let _profiles = store(&[("strata", strata_qwen_next())])?;
    let strata = target("strata", QWEN_NEXT)?;
    // @step When a turn is sent at effort "xhigh"
    let xhigh = sent(&strata, &effort("xhigh")?)?;
    // @step Then the request carries reasoning_effort "xhigh"
    assert_eq!(xhigh, json!({"reasoning_effort": "xhigh"}));
    // @step When a turn is sent at effort "none"
    let none = sent(&strata, &effort("none")?)?;
    // @step Then the request carries reasoning_effort "none"
    assert_eq!(none, json!({"reasoning_effort": "none"}));
    Ok(())
}

#[test]
#[serial]
fn an_effort_the_declaration_does_not_list_is_refused_not_left_for_the_engine_to_remap(
) -> TestResult {
    // @step Given a Strata profile whose model "qwen3.8-flash-next" declares efforts "none, low, medium, xhigh"
    let _profiles = store(&[("strata", strata_qwen_next())])?;
    let strata = target("strata", QWEN_NEXT)?;
    // @step When a turn is sent at effort "high"
    let reason = refused(&strata, &effort("high")?)?;
    // @step Then the turn is refused before the request is sent
    // (config_for refusing is the turn being refused before anything is built)
    // @step And the error names "qwen3.8-flash-next" and lists "none, low, medium, xhigh"
    assert!(
        reason.contains(QWEN_NEXT) && reason.contains("none, low, medium, xhigh"),
        "{reason}"
    );
    Ok(())
}

fn strata_deepseek(declaration: Value) -> Value {
    one_model("strata", "deepseek-v4.1", Some(declaration))
}

#[test]
#[serial]
fn a_whole_number_effort_is_sent_to_strata_as_a_json_number() -> TestResult {
    // @step Given a Strata profile whose DeepSeek-V4.1 model declares efforts "none, low, medium, high, max" and whole-number efforts from 1 to 100
    let _profiles = store(&[(
        "strata",
        strata_deepseek(json!({
            "efforts": ["none", "low", "medium", "high", "max"],
            "numericEfforts": whole(1, 100),
        })),
    )])?;
    let strata = target("strata", "deepseek-v4.1")?;
    // @step When a turn is sent at numeric effort 40
    let config = sent(&strata, &numeric(Number::from(40)))?;
    // @step Then the request carries reasoning_effort as the JSON number 40
    assert_eq!(config, json!({"reasoning_effort": 40}));
    assert!(config["reasoning_effort"].is_u64(), "{config}");
    Ok(())
}

#[test]
#[serial]
fn a_numeric_effort_outside_the_declared_range_or_of_the_wrong_kind_is_refused() -> TestResult {
    // @step Given a Strata profile whose DeepSeek-V4.1 model declares whole-number efforts from 1 to 100
    let _profiles = store(&[(
        "strata",
        strata_deepseek(json!({"numericEfforts": whole(1, 100)})),
    )])?;
    let strata = target("strata", "deepseek-v4.1")?;
    // @step When a turn is sent at numeric effort 150
    let reason = refused(&strata, &numeric(Number::from(150)))?;
    // @step Then the turn is refused before the request is sent, naming the model and the range 1 to 100
    assert!(
        reason.contains("deepseek-v4.1") && reason.contains("1 to 100"),
        "{reason}"
    );
    // @step When a turn is sent at numeric effort 40.5
    let fraction = Number::from_f64(40.5).or_fail("40.5 is a JSON number")?;
    let reason = refused(&strata, &numeric(fraction))?;
    // @step Then the turn is refused before the request is sent, saying the model takes whole numbers
    assert!(
        reason.contains("deepseek-v4.1") && reason.contains("whole number"),
        "{reason}"
    );
    Ok(())
}

#[test]
#[serial]
fn a_decimal_effort_is_sent_to_sglang_as_a_json_number() -> TestResult {
    // @step Given an SGLang profile whose model declares decimal efforts from 0.0 to 0.99
    let _profiles = store(&[(
        "sglang",
        one_model(
            "sglang",
            "inkling-1",
            Some(json!({"numericEfforts": decimal(0.0, 0.99)})),
        ),
    )])?;
    let sglang = target("sglang", "inkling-1")?;
    // @step When a turn is sent at numeric effort 0.5
    let half = Number::from_f64(0.5).or_fail("0.5 is a JSON number")?;
    let config = sent(&sglang, &numeric(half))?;
    // @step Then the request carries reasoning_effort as the JSON number 0.5
    assert_eq!(config, json!({"reasoning_effort": 0.5}));
    Ok(())
}

#[test]
#[serial]
fn a_declared_template_toggle_follows_the_chosen_effort() -> TestResult {
    // @step Given a vLLM profile whose DeepSeek-V3.1 model declares efforts "none, high" and the template toggle "thinking"
    let _profiles = store(&[(
        "vllm",
        one_model(
            "vllm",
            "deepseek-v3.1",
            Some(json!({"efforts": ["none", "high"], "templateToggle": "thinking"})),
        ),
    )])?;
    let vllm = target("vllm", "deepseek-v3.1")?;
    // @step When a turn is sent at effort "high"
    let high = sent(&vllm, &effort("high")?)?;
    // @step Then the request carries reasoning_effort "high" and chat_template_kwargs thinking true
    assert_eq!(
        high,
        json!({"reasoning_effort": "high", "chat_template_kwargs": {"thinking": true}})
    );
    // @step And chat_template_kwargs has no enable_thinking key
    assert!(high["chat_template_kwargs"]
        .get("enable_thinking")
        .is_none());
    // @step When a turn is sent at effort "none"
    let none = sent(&vllm, &effort("none")?)?;
    // @step Then the request carries reasoning_effort "none" and chat_template_kwargs thinking false
    assert_eq!(
        none,
        json!({"reasoning_effort": "none", "chat_template_kwargs": {"thinking": false}})
    );
    Ok(())
}

#[test]
#[serial]
fn a_model_with_no_declared_toggle_gets_no_template_kwargs() -> TestResult {
    // @step Given a vLLM profile whose Qwen3 model declares efforts "none, low, high" and no template toggle
    let _profiles = store(&[(
        "vllm",
        one_model("vllm", "qwen3-32b", Some(efforts(&["none", "low", "high"]))),
    )])?;
    let vllm = target("vllm", "qwen3-32b")?;
    // @step When a turn is sent at effort "low"
    let low = sent(&vllm, &effort("low")?)?;
    // @step Then the request carries reasoning_effort "low"
    assert_eq!(low["reasoning_effort"], json!("low"));
    // @step And the request carries no chat_template_kwargs, where MODEL-010 sent thinking and enable_thinking
    assert_eq!(low, json!({"reasoning_effort": "low"}));
    Ok(())
}

#[test]
#[serial]
fn a_toggle_only_model_offers_on_and_off_through_its_declared_key() -> TestResult {
    // @step Given a TensorRT-LLM profile whose model "qwen3-32b" declares only the template toggle "enable_thinking"
    let _profiles = store(&[(
        "trtllm",
        one_model(
            "tensorrt-llm",
            "qwen3-32b",
            Some(json!({"templateToggle": "enable_thinking"})),
        ),
    )])?;
    let manager = select("trtllm", "qwen3-32b")?;
    // @step Then the levels offered for "qwen3-32b" are "On, Off"
    assert_eq!(offered(&declared(&manager)?.capability), "On, Off");
    let trtllm =
        codelet_agent_loop::thinking_config::ThinkingTarget::for_provider_manager(&manager);
    // @step When a turn is sent at On
    let on = sent(&trtllm, &TurnLevel::Level(ReasoningLevel::On))?;
    // @step Then the request carries chat_template_kwargs enable_thinking true and no reasoning_effort
    assert_eq!(
        on,
        json!({"chat_template_kwargs": {"enable_thinking": true}})
    );
    // @step When a turn is sent at Off
    let off = sent(&trtllm, &TurnLevel::Off)?;
    // @step Then the request carries chat_template_kwargs enable_thinking false and no reasoning_effort
    assert_eq!(
        off,
        json!({"chat_template_kwargs": {"enable_thinking": false}})
    );
    Ok(())
}

#[test]
#[serial]
fn off_uses_the_declared_toggle_when_the_model_declares_no_none_effort() -> TestResult {
    // @step Given an LMDeploy profile whose model declares efforts "low, high" and the template toggle "enable_thinking"
    let _profiles = store(&[(
        "lmdeploy",
        one_model(
            "lmdeploy",
            "qwen3-8b",
            Some(json!({"efforts": ["low", "high"], "templateToggle": "enable_thinking"})),
        ),
    )])?;
    let lmdeploy = target("lmdeploy", "qwen3-8b")?;
    // @step When a turn is sent at Off
    let off = lmdeploy.config_for(Some(&TurnLevel::Off));
    // @step Then the request carries chat_template_kwargs enable_thinking false and no reasoning_effort
    // @step And it is not refused for lacking a none effort, as MODEL-010 refused it
    assert_eq!(
        off?,
        Some(json!({"chat_template_kwargs": {"enable_thinking": false}}))
    );
    Ok(())
}

#[test]
#[serial]
fn a_declared_token_budget_is_sent_in_the_engines_own_field() -> TestResult {
    let examples = [
        ("vllm", json!({"thinking_token_budget": 4096})),
        ("tensorrt-llm", json!({"thinking_token_budget": 4096})),
        ("llama-cpp", json!({"reasoning_budget_tokens": 4096})),
        ("strata", json!({"reasoning_budget_tokens": 4096})),
        (
            "sglang",
            json!({"custom_params": {"thinking_budget": 4096}}),
        ),
    ];
    for (engine, field) in examples {
        // @step Given a <engine> profile whose model declares a token budget from 128 to 32768
        let _profiles = store(&[(
            "local",
            one_model(
                engine,
                "qwen3-32b",
                Some(json!({"tokenBudget": {"min": 128, "max": 32768}})),
            ),
        )])?;
        let local = target("local", "qwen3-32b")?;
        // @step When a turn is sent at a 4096-token budget
        let budget = local.config_for(Some(&TurnLevel::Level(ReasoningLevel::Budget(4096))));
        // @step Then the request carries <field> 4096
        // @step And it is not refused as MODEL-010 refused budgets on local servers
        assert_eq!(budget?, Some(field), "engine {engine}");
    }
    Ok(())
}

#[test]
#[serial]
fn a_profile_wide_declaration_applies_to_a_model_with_none_of_its_own() -> TestResult {
    // @step Given a Strata profile that declares efforts "none, low, medium, xhigh" profile-wide
    // @step And its model "qwen3.8-flash-next" has no declaration of its own
    let _profiles = store(&[(
        "strata",
        profile(
            UNREACHABLE,
            Some("strata"),
            Some(efforts(&["none", "low", "medium", "xhigh"])),
            &[(QWEN_NEXT, None)],
        ),
    )])?;
    let strata = target("strata", QWEN_NEXT)?;
    // @step When a turn is sent at effort "medium"
    let medium = sent(&strata, &effort("medium")?)?;
    // @step Then the request carries reasoning_effort "medium"
    assert_eq!(medium, json!({"reasoning_effort": "medium"}));
    Ok(())
}

#[test]
#[serial]
fn a_per_model_declaration_replaces_the_profile_wide_one() -> TestResult {
    // @step Given a llama.cpp profile that declares efforts "low, high" profile-wide
    // @step And its model "gemma-4" declares efforts "none, low, medium, high, max"
    let _profiles = store(&[(
        "llama",
        profile(
            UNREACHABLE,
            Some("llama-cpp"),
            Some(json!({"efforts": ["low", "high"], "templateToggle": "enable_thinking"})),
            &[(
                "gemma-4",
                Some(efforts(&["none", "low", "medium", "high", "max"])),
            )],
        ),
    )])?;
    let manager = select("llama", "gemma-4")?;
    let gemma = declared(&manager)?;
    // @step Then the levels offered for "gemma-4" are "none, low, medium, high, max"
    assert_eq!(offered(&gemma.capability), "none, low, medium, high, max");
    // @step And nothing from the profile-wide declaration is merged in
    assert!(!gemma.capability.has_toggle());
    Ok(())
}

#[test]
#[serial]
fn an_undeclared_local_model_has_no_reasoning_control() -> TestResult {
    // @step Given a vLLM profile whose model "qwen3-32b" has no reasoning declaration, profile-wide or per model
    let _profiles = store(&[("vllm", one_model("vllm", "qwen3-32b", None))])?;
    let vllm = target("vllm", "qwen3-32b")?;
    // @step When a turn is sent at effort "high"
    let reason = refused(&vllm, &effort("high")?)?;
    // @step Then the turn is refused before the request is sent
    // @step And the error names "qwen3-32b" and says to declare its levels in the profile
    assert!(
        reason.contains("qwen3-32b") && reason.contains("declare its levels in the profile"),
        "{reason}"
    );
    // @step And neither reasoning_effort nor chat_template_kwargs is sent, where MODEL-010 sent both unchecked
    let off = refused(&vllm, &TurnLevel::Off)?;
    assert!(off.contains("declare its levels in the profile"), "{off}");
    Ok(())
}

fn chat_completion() -> Value {
    json!({
        "id": "chatcmpl-model016",
        "object": "chat.completion",
        "created": 0,
        "model": "gemma-4",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "5"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
}

#[tokio::test]
#[serial]
async fn lm_studio_receives_reasoning_effort_on_chat_completions() -> TestResult {
    // @step Given an LM Studio profile whose model declares efforts "low, medium, high"
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion()))
        .mount(&server)
        .await;
    let base_url = format!("{}/v1", server.uri());
    let _profiles = store(&[(
        "lmstudio",
        profile(
            &base_url,
            Some("lm-studio"),
            None,
            &[("gemma-4", Some(efforts(&["low", "medium", "high"])))],
        ),
    )])?;
    let lmstudio = target("lmstudio", "gemma-4")?;
    // @step When a turn is sent at effort "medium"
    let thinking = sent(&lmstudio, &effort("medium")?)?;
    let provider =
        OpenAIProvider::from_api_key_with_options("test", "gemma-4", Some(&base_url), None)?;
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, Some(thinking));
    agent
        .completion("what is 3 + 2", vec![])
        .await?
        .send()
        .await?;
    // @step Then the /v1/chat/completions request carries reasoning_effort "medium"
    let requests = server
        .received_requests()
        .await
        .or_fail("requests were recorded")?;
    let request = requests.first().or_fail("the server received a request")?;
    let body: Value = serde_json::from_slice(&request.body)?;
    assert_eq!(body["reasoning_effort"], json!("medium"), "{body}");
    assert!(body.get("chat_template_kwargs").is_none(), "{body}");
    Ok(())
}

#[test]
#[serial]
fn a_declared_part_the_engine_has_no_field_for_is_an_error_for_that_model_only() -> TestResult {
    let examples = [
        ("tgi", "TGI", efforts(&["low", "high"]), "efforts"),
        (
            "ollama",
            "Ollama",
            json!({"templateToggle": "enable_thinking"}),
            "template toggle",
        ),
        (
            "lmdeploy",
            "LMDeploy",
            json!({"tokenBudget": {"min": 128, "max": 32768}}),
            "token budget",
        ),
        (
            "lm-studio",
            "LM Studio",
            json!({"tokenBudget": {"min": 128, "max": 32768}}),
            "token budget",
        ),
        (
            "strata",
            "Strata",
            json!({"templateToggle": "thinking"}),
            "template toggle thinking",
        ),
        (
            "vllm",
            "vLLM",
            json!({"numericEfforts": decimal(0.0, 0.99)}),
            "decimal efforts",
        ),
    ];
    for (engine, name, declaration, part) in examples {
        // @step Given a <engine> profile whose model declares <part>
        // @step And a vLLM profile whose model declares efforts "low, high"
        let _profiles = store(&[
            ("local", one_model(engine, "local-model", Some(declaration))),
            (
                "vllm",
                one_model("vllm", "qwen3-32b", Some(efforts(&["low", "high"]))),
            ),
        ])?;
        // @step When the profiles are loaded
        let local = loaded("local")?;
        let vllm = loaded("vllm")?;
        // @step Then the <engine> model's declaration is an error naming the profile, the model, <engine> and <part>
        let reason = match local.model_reasoning("local-model") {
            Ok(reasoning) => {
                return Err(format!("{engine}: expected an error, got {reasoning:?}").into())
            }
            Err(reason) => reason,
        };
        assert!(
            reason.contains("profile local")
                && reason.contains("local-model")
                && reason.contains(name)
                && reason.contains(part),
            "{reason}"
        );
        // @step And the vLLM model still offers "low, high"
        let qwen = vllm
            .model_reasoning("qwen3-32b")?
            .or_fail("the vLLM model has a declaration")?;
        assert_eq!(offered(&qwen.capability), "low, high");
    }
    Ok(())
}

#[test]
#[serial]
fn a_declaration_on_a_profile_with_no_engine_is_an_error() -> TestResult {
    // @step Given a local-server profile with no engine whose model declares efforts "low, high"
    let _profiles = store(&[(
        "ollama-box",
        profile(
            "http://localhost:11434/v1",
            None,
            None,
            &[("llama-cpp-gemma", Some(efforts(&["low", "high"])))],
        ),
    )])?;
    // @step When the profile is loaded
    let profile = loaded("ollama-box")?;
    // @step Then the declaration is an error naming the profile and saying it names no engine
    let reason = match profile.model_reasoning("llama-cpp-gemma") {
        Ok(reasoning) => return Err(format!("expected an error, got {reasoning:?}").into()),
        Err(reason) => reason,
    };
    assert!(
        reason.contains("ollama-box") && reason.contains("names no engine"),
        "{reason}"
    );
    // @step And no engine is inferred from the base URL or the model id
    let target = target("ollama-box", "llama-cpp-gemma")?;
    let refusal = refused(&target, &effort("low")?)?;
    assert!(refusal.contains("names no engine"), "{refusal}");
    Ok(())
}

/// A live server answering `verb path` with `body`; the profile's base URL.
async fn live(verb: &str, route: &str, body: Value) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(method(verb))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let base_url = format!("{}/v1", server.uri());
    (server, base_url)
}

fn props(levels: &[&str]) -> Value {
    json!({"chat_template_caps": {"supports_reasoning_effort": true, "reasoning_effort_levels": levels}})
}

/// Resolve `profile/model`'s capability and run its live check.
async fn resolve_and_check(
    profile: &str,
    model: &str,
) -> Result<(codelet_providers::ProviderManager, LiveCheckReport), codelet_test_helpers::TestError>
{
    let mut manager = select(profile, model)?;
    let check = manager
        .take_live_check()
        .or_fail("the engine has a live endpoint")?;
    let report = check.run().await;
    Ok((manager, report))
}

fn mismatch(report: LiveCheckReport) -> Result<String, codelet_test_helpers::TestError> {
    match report {
        LiveCheckReport::Mismatch(message) => Ok(message),
        other => Err(format!("expected a mismatch, got {other:?}").into()),
    }
}

#[tokio::test]
#[serial]
async fn a_live_report_that_differs_from_the_declaration_is_reported_and_the_declaration_still_applies(
) -> TestResult {
    // @step Given a llama.cpp profile whose model declares efforts "none, low, high, max"
    // @step And its GET /props reports reasoning_effort_levels "low, medium, high"
    let (_server, base_url) = live("GET", "/props", props(&["low", "medium", "high"])).await;
    let _profiles = store(&[(
        "llama",
        profile(
            &base_url,
            Some("llama-cpp"),
            None,
            &[("gemma-4", Some(efforts(&["none", "low", "high", "max"])))],
        ),
    )])?;
    // @step When the model's capability is resolved
    let (manager, report) = resolve_and_check("llama", "gemma-4").await?;
    // @step Then a mismatch is reported naming the model, the declared "none, low, high, max" and the reported "low, medium, high"
    let message = mismatch(report)?;
    assert!(
        message.contains("gemma-4")
            && message.contains("none, low, high, max")
            && message.contains("low, medium, high"),
        "{message}"
    );
    // @step And the levels offered are "none, low, high, max"
    assert_eq!(
        offered(&declared(&manager)?.capability),
        "none, low, high, max"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_declared_none_is_not_a_mismatch_against_llama_cpp() -> TestResult {
    // @step Given a llama.cpp profile whose model declares efforts "none, low, high"
    // @step And its GET /props reports reasoning_effort_levels "low, high"
    let (_server, base_url) = live("GET", "/props", props(&["low", "high"])).await;
    let _profiles = store(&[(
        "llama",
        profile(
            &base_url,
            Some("llama-cpp"),
            None,
            &[("gemma-4", Some(efforts(&["none", "low", "high"])))],
        ),
    )])?;
    // @step When the model's capability is resolved
    let (_manager, report) = resolve_and_check("llama", "gemma-4").await?;
    // @step Then no mismatch is reported
    assert_eq!(report, LiveCheckReport::Agrees);
    Ok(())
}

#[tokio::test]
#[serial]
async fn live_data_never_stands_in_for_a_missing_declaration() -> TestResult {
    // @step Given a llama.cpp profile whose model has no reasoning declaration
    // @step And its GET /props reports reasoning_effort_levels "low, high"
    let (_server, base_url) = live("GET", "/props", props(&["low", "high"])).await;
    let _profiles = store(&[(
        "llama",
        profile(&base_url, Some("llama-cpp"), None, &[("gemma-4", None)]),
    )])?;
    let mut manager = select("llama", "gemma-4")?;
    assert!(manager.take_live_check().is_none());
    let llama = codelet_agent_loop::thinking_config::ThinkingTarget::for_provider_manager(&manager);
    // @step When a turn is sent at effort "low"
    let reason = refused(&llama, &effort("low")?)?;
    // @step Then the turn is refused before the request is sent, saying to declare the model's levels in the profile
    assert!(
        reason.contains("gemma-4") && reason.contains("declare its levels in the profile"),
        "{reason}"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_live_check_that_cannot_run_is_reported_and_the_declaration_still_applies() -> TestResult
{
    // @step Given an Ollama profile whose model declares efforts "none, low, medium, high"
    // @step And POST /api/show cannot be reached
    let _profiles = store(&[(
        "ollama",
        one_model(
            "ollama",
            "qwen3:8b",
            Some(efforts(&["none", "low", "medium", "high"])),
        ),
    )])?;
    // @step When the model's capability is resolved
    let (manager, report) = resolve_and_check("ollama", "qwen3:8b").await?;
    // @step Then a failed live check is reported naming the model and POST /api/show
    let message = match report {
        LiveCheckReport::Failed(message) => message,
        other => return Err(format!("expected a failed check, got {other:?}").into()),
    };
    assert!(
        message.contains("qwen3:8b") && message.contains("POST /api/show"),
        "{message}"
    );
    // @step And the levels offered are "none, low, medium, high"
    assert_eq!(
        offered(&declared(&manager)?.capability),
        "none, low, medium, high"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn ollama_false_matches_a_declared_none() -> TestResult {
    // @step Given an Ollama profile whose model declares efforts "none, low, medium, high"
    // @step And its POST /api/show reports thinking values "false, low, medium, high"
    let (_server, base_url) = live(
        "POST",
        "/api/show",
        json!({"thinking": {"values": [false, "low", "medium", "high"], "default": "medium"}}),
    )
    .await;
    let _profiles = store(&[(
        "ollama",
        profile(
            &base_url,
            Some("ollama"),
            None,
            &[(
                "qwen3:8b",
                Some(efforts(&["none", "low", "medium", "high"])),
            )],
        ),
    )])?;
    // @step When the model's capability is resolved
    let (_manager, report) = resolve_and_check("ollama", "qwen3:8b").await?;
    // @step Then no mismatch is reported
    assert_eq!(report, LiveCheckReport::Agrees);
    Ok(())
}

#[tokio::test]
#[serial]
async fn lm_studios_off_and_on_are_compared_with_the_declaration_but_change_nothing_sent(
) -> TestResult {
    // @step Given an LM Studio profile whose model declares efforts "low, medium, high"
    // @step And its GET /api/v1/models reports allowed_options "off, on"
    let (_server, base_url) = live(
        "GET",
        "/api/v1/models",
        json!({"models": [{
            "type": "llm",
            "key": "google/gemma-4",
            "loaded_instances": [{"id": "google/gemma-4"}],
            "capabilities": {"reasoning": {"allowed_options": ["off", "on"], "default": "on"}}
        }]}),
    )
    .await;
    let _profiles = store(&[(
        "lmstudio",
        profile(
            &base_url,
            Some("lm-studio"),
            None,
            &[("google/gemma-4", Some(efforts(&["low", "medium", "high"])))],
        ),
    )])?;
    // @step When the model's capability is resolved
    let (manager, report) = resolve_and_check("lmstudio", "google/gemma-4").await?;
    // @step Then a mismatch is reported naming the model, the declared "low, medium, high" and the reported "off, on"
    let message = mismatch(report)?;
    assert!(
        message.contains("google/gemma-4")
            && message.contains("low, medium, high")
            && message.contains("off, on"),
        "{message}"
    );
    let lmstudio =
        codelet_agent_loop::thinking_config::ThinkingTarget::for_provider_manager(&manager);
    // @step When a turn is sent at effort "medium"
    let medium = sent(&lmstudio, &effort("medium")?)?;
    // @step Then the request carries reasoning_effort "medium"
    assert_eq!(medium, json!({"reasoning_effort": "medium"}));
    Ok(())
}
