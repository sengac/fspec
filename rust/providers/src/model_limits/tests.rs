use super::*;

// -- Test doubles -------------------------------------------------------

/// A resolver for an endpoint with genuine hard ceilings.
struct ClampingResolver {
    max_ctx: Option<usize>,
    default_ctx: usize,
    max_out: Option<usize>,
    default_out: usize,
    send_max_output: bool,
}

impl ClampingResolver {
    fn new() -> Self {
        Self {
            max_ctx: Some(200_000),
            default_ctx: 200_000,
            max_out: Some(8_192),
            default_out: 8_192,
            send_max_output: true,
        }
    }

    fn trusting() -> Self {
        Self {
            max_ctx: None,
            default_ctx: 128_000,
            max_out: None,
            default_out: 4_096,
            send_max_output: true,
        }
    }

    fn codex_like() -> Self {
        Self {
            max_ctx: None,
            default_ctx: 272_000,
            max_out: None,
            default_out: 4_096,
            send_max_output: false,
        }
    }
}

impl ModelLimitsResolver for ClampingResolver {
    fn max_context_window(&self) -> Option<usize> {
        self.max_ctx
    }

    fn max_output_tokens_limit(&self) -> Option<usize> {
        self.max_out
    }

    fn default_context_window(&self) -> Option<usize> {
        Some(self.default_ctx)
    }

    fn default_max_output_tokens(&self) -> Option<usize> {
        Some(self.default_out)
    }

    fn should_send_max_output_tokens(&self) -> bool {
        self.send_max_output
    }
}

// -- Scenario tests -----------------------------------------------------

#[test]
fn provider_clamps_registry_value_to_its_hard_maximum() {
    // @step Given a provider declares max_context_window as 200000
    let resolver = ClampingResolver::new();

    // @step And the registry reports a context window of 1000000
    let registry_value = Some(1_000_000);

    // @step And no user override is set
    let user_override = None;

    // @step When the model limits are resolved
    let result = resolve_context_window(registry_value, user_override, &resolver);

    // @step Then the resolved context window should be 200000
    assert_eq!(result, Some(200_000));
}

#[test]
fn provider_trusts_registry_value_when_no_max_is_declared() {
    // @step Given a provider declares max_context_window as None
    let resolver = ClampingResolver::trusting();

    // @step And the registry reports a context window of 128000
    let registry_value = Some(128_000);

    // @step And no user override is set
    let user_override = None;

    // @step When the model limits are resolved
    let result = resolve_context_window(registry_value, user_override, &resolver);

    // @step Then the resolved context window should be 128000
    assert_eq!(result, Some(128_000));
}

#[test]
fn user_override_is_clamped_by_provider_max() {
    // @step Given a provider declares max_context_window as 200000
    let resolver = ClampingResolver::new();

    // @step And no registry value is available
    let registry_value = None;

    // @step And the user override is set to 500000
    let user_override = Some(500_000);

    // @step When the model limits are resolved
    let result = resolve_context_window(registry_value, user_override, &resolver);

    // @step Then the resolved context window should be 200000
    assert_eq!(result, Some(200_000));
}

#[test]
fn user_override_is_trusted_when_provider_declares_no_max() {
    // @step Given a provider declares max_context_window as None
    let resolver = ClampingResolver::trusting();

    // @step And no registry value is available
    let registry_value = None;

    // @step And the user override is set to 100000
    let user_override = Some(100_000);

    // @step When the model limits are resolved
    let result = resolve_context_window(registry_value, user_override, &resolver);

    // @step Then the resolved context window should be 100000
    assert_eq!(result, Some(100_000));
}

#[test]
fn provider_default_is_used_when_no_registry_or_user_override_exists() {
    // @step Given a provider declares default_context_window as 272000
    let resolver = ClampingResolver::codex_like();

    // @step And no registry value is available
    let registry_value = None;

    // @step And no user override is set
    let user_override = None;

    // @step When the model limits are resolved
    let result = resolve_context_window(registry_value, user_override, &resolver);

    // @step Then the resolved context window should be 272000
    assert_eq!(result, Some(272_000));
}

#[test]
fn provider_can_suppress_sending_max_output_tokens() {
    // @step Given a provider declares should_send_max_output_tokens as false
    let resolver = ClampingResolver::codex_like();

    // @step Then the resolver should indicate max_output_tokens must not be sent
    assert!(!resolver.should_send_max_output_tokens());
}

#[test]
fn max_output_tokens_are_clamped_by_provider_limit() {
    // @step Given a provider declares max_output_tokens_limit as 8192
    let resolver = ClampingResolver::new();

    // @step And the registry reports max output tokens of 128000
    let registry_value = Some(128_000);

    // @step And no user override is set
    let user_override = None;

    // @step When the model output limits are resolved
    let result = resolve_max_output_tokens(registry_value, user_override, &resolver);

    // @step Then the resolved max output tokens should be 8192
    assert_eq!(result, Some(8_192));
}

#[test]
fn provider_default_max_output_tokens_used_when_no_registry_data() {
    // @step Given a provider declares default_max_output_tokens as 4096
    let resolver = ClampingResolver::codex_like();

    // @step And no registry value is available
    let registry_value = None;

    // @step And no user override is set
    let user_override = None;

    // @step When the model output limits are resolved
    let result = resolve_max_output_tokens(registry_value, user_override, &resolver);

    // @step Then the resolved max output tokens should be 4096
    assert_eq!(result, Some(4_096));
}

// -- Default trait method tests -----------------------------------------

#[test]
fn should_send_max_output_tokens_defaults_to_true() {
    struct MinimalResolver;
    impl ModelLimitsResolver for MinimalResolver {
        fn default_context_window(&self) -> Option<usize> {
            Some(128_000)
        }
        fn default_max_output_tokens(&self) -> Option<usize> {
            Some(4_096)
        }
    }
    let resolver = MinimalResolver;
    assert!(resolver.should_send_max_output_tokens());
}

#[test]
fn max_context_window_defaults_to_none() {
    struct MinimalResolver;
    impl ModelLimitsResolver for MinimalResolver {
        fn default_context_window(&self) -> Option<usize> {
            Some(128_000)
        }
        fn default_max_output_tokens(&self) -> Option<usize> {
            Some(4_096)
        }
    }
    let resolver = MinimalResolver;
    assert_eq!(resolver.max_context_window(), None);
}

#[test]
fn max_output_tokens_limit_defaults_to_none() {
    struct MinimalResolver;
    impl ModelLimitsResolver for MinimalResolver {
        fn default_context_window(&self) -> Option<usize> {
            Some(128_000)
        }
        fn default_max_output_tokens(&self) -> Option<usize> {
            Some(4_096)
        }
    }
    let resolver = MinimalResolver;
    assert_eq!(resolver.max_output_tokens_limit(), None);
}
