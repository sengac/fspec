use rig::completion::{CompletionError, GetTokenUsage};
use rig::providers::openai::responses_api::ResponsesUsage;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

trait AggregationResult {
    fn into_result(self) -> Result<ResponsesUsage, CompletionError>;
}

impl AggregationResult for ResponsesUsage {
    fn into_result(self) -> Result<ResponsesUsage, CompletionError> {
        Ok(self)
    }
}

impl AggregationResult for Result<ResponsesUsage, CompletionError> {
    fn into_result(self) -> Result<ResponsesUsage, CompletionError> {
        self
    }
}

fn usage(input: u64, output: u64, total: u64) -> Result<ResponsesUsage, serde_json::Error> {
    serde_json::from_value(json!({
        "input_tokens": input, "output_tokens": output, "total_tokens": total,
        "input_tokens_details": {"cached_tokens": 2},
        "output_tokens_details": {"reasoning_tokens": 1}
    }))
}

/// Feature: spec/features/builtin-provider-responses-usage-aggregation.feature
#[test]
fn invalid_physical_usage_cannot_become_valid_in_aggregate() -> TestResult {
    // @step Given two physical Responses measurements whose inconsistent totals cancel
    let left = usage(10, 5, 14)?;
    let right = usage(10, 5, 16)?;
    assert!(left.token_usage().is_none());
    assert!(right.token_usage().is_none());
    // @step When their usage is aggregated
    // @step Then aggregation rejects the invalid physical evidence
    assert!((left + right).into_result().is_err());
    Ok(())
}

/// Feature: spec/features/builtin-provider-responses-usage-aggregation.feature
#[test]
fn individually_valid_usage_cannot_overflow_its_aggregate() -> TestResult {
    // @step Given individually valid Responses measurements whose combined totals exceed the integer limit
    for (left, right) in [
        (usage(u64::MAX - 1, 1, u64::MAX)?, usage(2, 1, 3)?),
        (usage(2, u64::MAX - 2, u64::MAX)?, usage(2, 1, 3)?),
        (usage(u64::MAX - 4, 1, u64::MAX - 3)?, usage(2, 3, 5)?),
    ] {
        assert!(left.token_usage().is_some());
        assert!(right.token_usage().is_some());
        // @step When their usage is aggregated
        let result = std::panic::catch_unwind(|| (left + right).into_result());
        // @step Then aggregation returns an error without panic or wrapped counters
        assert!(result.is_ok(), "aggregation must not panic on overflow");
        assert!(result.map_err(|_| "aggregation panicked")?.is_err());
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-responses-usage-aggregation.feature
#[test]
fn missing_detail_remains_unknown_in_either_operand_order() -> TestResult {
    // @step Given valid Responses measurements with absent cache and reasoning detail
    let known = usage(10, 5, 15)?;
    let mut cache_unknown = known.clone();
    cache_unknown.input_tokens_details = None;
    // @step When they are aggregated with explicitly measured detail in either order
    for (left, right) in [
        (known.clone(), cache_unknown.clone()),
        (cache_unknown, known.clone()),
    ] {
        let aggregate = (left + right).into_result()?;
        assert!(
            aggregate
                .token_usage()
                .ok_or("invalid aggregate")?
                .cache_read_input_tokens
                .is_none()
        );
    }
    let unknown: ResponsesUsage = serde_json::from_value(json!({
        "input_tokens": 10, "output_tokens": 5, "total_tokens": 15
    }))?;
    for (left, right) in [(known.clone(), unknown.clone()), (unknown, known)] {
        let aggregate = (left + right).into_result()?;
        let tokens = aggregate.token_usage().ok_or("invalid aggregate")?;
        // @step Then both aggregate details remain unknown
        assert!(tokens.cache_read_input_tokens.is_none());
        assert!(tokens.reasoning_tokens.is_none());
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-responses-usage-aggregation.feature
#[test]
fn explicit_detail_and_inclusive_totals_aggregate_exactly() -> TestResult {
    // @step Given valid Responses measurements with explicit cache and reasoning detail
    // @step When their usage is aggregated
    let aggregate = (usage(10, 5, 15)? + usage(20, 8, 28)?).into_result()?;
    // @step Then inclusive totals and measured subsets sum exactly without double counting
    assert_eq!(aggregate.input_tokens, 30);
    assert_eq!(aggregate.output_tokens, 13);
    assert_eq!(aggregate.total_tokens, 43);
    let tokens = aggregate.token_usage().ok_or("invalid aggregate")?;
    assert_eq!(tokens.input_tokens, 26);
    assert_eq!(tokens.output_tokens, 11);
    assert_eq!(tokens.cache_read_input_tokens, Some(4));
    assert_eq!(tokens.reasoning_tokens, Some(2));
    Ok(())
}
