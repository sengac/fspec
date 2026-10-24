//! Feature: spec/features/inclusive-provider-usage.feature

use codelet_core::ApiTokenUsage;
use rig::completion::GetTokenUsage;
use serde_json::{json, Value};

fn context(usage: &rig::completion::Usage) -> u64 {
    let mut accounted = ApiTokenUsage::default();
    accounted.update_from_usage(usage);
    accounted.total_context()
}

proptest::proptest! {
    #[test]
    fn inclusive_subset_normalization_preserves_detail_across_valid_counts(
        input in 0_u32..1_000_000, output in 0_u32..100_000,
        cache_fraction in 0_u32..101, reasoning_fraction in 0_u32..101,
    ) {
        // @step Given valid inclusive usage counts and their cache and reasoning subsets
        let cached = u64::from(input) * u64::from(cache_fraction) / 100;
        let reasoning = u64::from(output) * u64::from(reasoning_fraction) / 100;
        let raw = openai(json!({
            "prompt_tokens": input, "completion_tokens": output,
            "total_tokens": u64::from(input) + u64::from(output),
            "prompt_tokens_details": {"cached_tokens": cached},
            "completion_tokens_details": {"reasoning_tokens": reasoning}
        }))?;
        // @step When usage is normalized and serialized for transport
        let usage = raw.token_usage().ok_or_else(|| proptest::test_runner::TestCaseError::fail("valid subset usage"))?;
        // @step Then occupancy equals inclusive input plus output for every count
        proptest::prop_assert_eq!(context(&usage), u64::from(input) + u64::from(output));
        let restored: rig::completion::Usage = serde_json::from_value(
            serde_json::to_value(usage).map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?
        ).map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        // @step And restored usage preserves measured subset detail
        proptest::prop_assert_eq!(restored.cache_read_input_tokens, Some(cached));
        proptest::prop_assert_eq!(restored.reasoning_tokens, Some(reasoning));
    }
}

fn openai(
    raw: Value,
) -> Result<
    rig::providers::openai::completion::streaming::StreamingCompletionResponse,
    serde_json::Error,
> {
    serde_json::from_value(json!({"usage": raw}))
}

fn responses(
    raw: Value,
) -> Result<
    rig::providers::openai::responses_api::streaming::StreamingCompletionResponse,
    serde_json::Error,
> {
    serde_json::from_value(json!({"usage": raw}))
}

#[test]
fn inclusive_provider_subsets_count_once() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an OpenAI-compatible or Codex request reports input 1000 including cached 400 and output 100 including reasoning 30
    let chat = openai(json!({
        "prompt_tokens": 1000, "completion_tokens": 100, "total_tokens": 1100,
        "prompt_tokens_details": {"cached_tokens": 400},
        "completion_tokens_details": {"reasoning_tokens": 30}
    }))?;
    let codex = responses(json!({
        "input_tokens": 1000, "output_tokens": 100, "total_tokens": 1100,
        "input_tokens_details": {"cached_tokens": 400},
        "output_tokens_details": {"reasoning_tokens": 30}
    }))?;
    // @step When the provider usage reaches context accounting
    let normalized = [chat.token_usage(), codex.token_usage()];
    // @step Then occupancy is 1100 and measured cache and reasoning detail remains available
    for usage in normalized {
        let usage = usage.ok_or("consistent inclusive usage")?;
        assert_eq!(context(&usage), 1100);
        assert_eq!(usage.cache_read_input_tokens, Some(400));
        assert_eq!(usage.reasoning_tokens, Some(30));
        let encoded = serde_json::to_string(&usage)?;
        let decoded: rig::completion::Usage = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, usage);
    }
    Ok(())
}

#[test]
fn anthropic_disjoint_categories_retain_full_occupancy() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an Anthropic request reports fresh input 600 cache read 400 cache creation 50 and output 100
    let raw: rig::providers::anthropic::streaming::StreamingCompletionResponse =
        serde_json::from_value(json!({"usage": {
            "input_tokens": 600, "output_tokens": 100,
            "cache_read_input_tokens": 400, "cache_creation_input_tokens": 50
        }}))?;
    // @step When the provider usage reaches context accounting
    let usage = raw.token_usage().ok_or("measured Anthropic usage")?;
    // @step Then occupancy is 1150 and all measured cache categories remain intact
    assert_eq!(context(&usage), 1150);
    assert_eq!(usage.cache_read_input_tokens, Some(400));
    assert_eq!(usage.cache_creation_input_tokens, Some(50));
    Ok(())
}

#[test]
fn gemini_live_and_final_usage_preserve_the_same_occupancy(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a Gemini request reports prompt 1000 cached 400 candidates 70 thoughts 30 and total 1100
    let raw = json!({
        "promptTokenCount": 1000, "cachedContentTokenCount": 400,
        "candidatesTokenCount": 70, "thoughtsTokenCount": 30, "totalTokenCount": 1100
    });
    let live: rig::providers::gemini::streaming::PartialUsage =
        serde_json::from_value(raw.clone())?;
    let final_response: rig::providers::gemini::streaming::StreamingCompletionResponse =
        serde_json::from_value(json!({"usage_metadata": raw}))?;
    // @step When live and final provider usage reach context accounting
    let normalized = [live.token_usage(), final_response.token_usage()];
    // @step Then both report occupancy 1100 and preserve cached 400 and reasoning 30
    for usage in normalized {
        let usage = usage.ok_or("consistent Gemini usage")?;
        assert_eq!(context(&usage), 1100);
        assert_eq!(usage.cache_read_input_tokens, Some(400));
        assert_eq!(usage.reasoning_tokens, Some(30));
    }
    Ok(())
}

#[test]
fn inconsistent_provider_subsets_remain_unknown() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a provider reports cached or reasoning subsets greater than their inclusive parent counts
    let fixtures = [(1001, 30), (400, 101)];
    // @step When normalization is requested
    for (cached, reasoning) in fixtures {
        let raw = openai(json!({
            "prompt_tokens": 1000, "completion_tokens": 100, "total_tokens": 1100,
            "prompt_tokens_details": {"cached_tokens": cached},
            "completion_tokens_details": {"reasoning_tokens": reasoning}
        }))?;
        // @step Then no authoritative normalized usage is returned and raw detail is not rewritten as a measured zero
        assert!(raw.token_usage().is_none());
        assert_eq!(
            raw.usage
                .prompt_tokens_details
                .ok_or("raw cache detail")?
                .cached_tokens,
            cached
        );
    }
    Ok(())
}
