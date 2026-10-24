use codelet_providers::ProviderRuntimeOptions;
use proptest::prelude::*;
use serde_json::json;

proptest! {
    #[test]
    fn optional_runtime_settings_preserve_camel_case_and_round_trip(
        streaming in proptest::option::of(any::<bool>()),
        preserve_thinking in proptest::option::of(any::<bool>()),
        loop_enabled in proptest::option::of(any::<bool>()),
        continuation in proptest::option::of(any::<u32>()),
        images in proptest::option::of(any::<u32>()),
        window in proptest::option::of(1_u32..=u32::MAX),
        repeats in proptest::option::of(1_u32..=u32::MAX),
        retries in proptest::option::of(any::<u32>()),
        context in proptest::option::of(1_usize..=u32::MAX as usize),
        output in proptest::option::of(1_usize..=u32::MAX as usize),
        threshold in proptest::option::of(1_u64..=100),
    ) {
        let input = json!({
            "streaming":streaming, "preserveThinking":preserve_thinking,
            "loopDetectionEnabled":loop_enabled, "autoContinue":continuation,
            "maxImages":images, "loopDetectionWindow":window,
            "loopDetectionMaxRepeats":repeats, "loopDetectionMaxRetries":retries,
            "contextWindow":context, "maxOutputTokens":output,
            "compactionThreshold":threshold.map(|value| json!({"type":"percentage","value":value}))
        });
        let options: ProviderRuntimeOptions = serde_json::from_value(input)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(options.streaming, streaming);
        prop_assert_eq!(options.preserve_thinking, preserve_thinking);
        prop_assert_eq!(options.loop_detection_enabled, loop_enabled);
        prop_assert_eq!(options.auto_continue, continuation);
        prop_assert_eq!(options.max_images, images);
        prop_assert_eq!(options.loop_detection_window, window);
        prop_assert_eq!(options.loop_detection_max_repeats, repeats);
        prop_assert_eq!(options.loop_detection_max_retries, retries);
        prop_assert_eq!(options.context_window, context);
        prop_assert_eq!(options.max_output_tokens, output);
        let encoded = serde_json::to_value(&options)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(encoded["compactionThreshold"]["value"].as_u64(), threshold);
        for field in ["preserve_thinking", "auto_continue", "context_window", "max_output_tokens"] {
            prop_assert!(encoded.get(field).is_none());
        }
        let decoded: ProviderRuntimeOptions = serde_json::from_value(encoded)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(decoded, options);
    }
}
