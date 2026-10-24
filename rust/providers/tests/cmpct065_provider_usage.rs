#[path = "../../core/tests/cmpct065_provider_usage.rs"]
mod provider_usage;

use rig::completion::GetTokenUsage;

#[test]
fn custom_providers_declare_their_additive_usage_contract() -> Result<(), Box<dyn std::error::Error>>
{
    use codelet_providers::custom::rig_model::RhaiCustomCompletion;
    // @step Given a custom provider reports disjoint input cache output and reasoning categories
    let raw = RhaiCustomCompletion {
        input_tokens: 600,
        output_tokens: 70,
        cache_read_input_tokens: Some(400),
        cache_creation_input_tokens: Some(50),
        reasoning_tokens: Some(30),
        ..Default::default()
    };
    // @step When custom usage reaches context accounting
    let usage = raw.token_usage().ok_or("missing declared custom usage")?;
    let mut accounted = codelet_core::ApiTokenUsage::default();
    accounted.update_from_usage(&usage);
    // @step Then each category contributes once and missing measurements remain unknown
    assert_eq!(accounted.total_context(), 1150);
    assert!(RhaiCustomCompletion::default().token_usage().is_none());
    Ok(())
}
