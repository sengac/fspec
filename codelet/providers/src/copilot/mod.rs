//! GitHub Copilot Provider Module (PROV-053/054/055/056)
//!
//! Implements GitHub Copilot as a fspec LLM provider with:
//! - OAuth 2.0 device authorization flow (RFC 8628) — PROV-054
//! - Token persistence at `~/.fspec/credentials/copilot_auth.json` (mode 0600) — PROV-054
//! - `CopilotHttpClient` HTTP middleware, facades & endpoint routing — PROV-055
//! - Live model catalog fetched from the Copilot `/models` endpoint — PROV-056
//!
//! ## Module layout
//!
//! | File                   | Responsibility                                            |
//! |------------------------|-----------------------------------------------------------|
//! | `auth.rs`              | Credential file persistence (`copilot_auth.json`)          |
//! | `oauth.rs`             | Device authorization flow orchestrator                    |
//! | `constants.rs`         | Shared identifiers and header literal values              |
//! | `header_facade.rs`     | `CopilotHeaderFacade::build_headers` (pure)               |
//! | `classifier.rs`        | `CopilotRequestClassifier::classify` (pure)               |
//! | `endpoint.rs`          | `CopilotEndpointFacade::select` routing (pure)            |
//! | `behavior_facade.rs`   | `CopilotBehaviorFacade` trait + 3 impls + selector        |
//! | `refreshing_client.rs` | `CopilotHttpClient` middleware (`HttpClientExt` impl)     |
//! | `provider.rs`          | `CopilotProvider` composition layer (`LlmProvider` impl)  |
//! | `provider_options.rs`  | `apply_store_false` zero-retention enforcement            |
//! | `models/`              | Model catalog fetch + wire-format → domain mapping        |
//!
//! Enterprise deployments are supported via the
//! [`oauth::CopilotDeploymentType::Enterprise`] variant, which routes
//! device-code requests to `https://<host>/login/device/code` instead of
//! `https://github.com/login/device/code`.

pub mod auth;
pub mod base_url;
pub mod behavior_facade;
pub mod classifier;
pub mod constants;
pub mod endpoint;
pub mod header_facade;
pub mod model_family;
pub mod models;
pub mod oauth;
pub mod oauth_device_code;
pub mod oauth_polling;
pub mod oauth_types;
pub mod prompt_cache;
pub mod provider;
pub mod provider_options;
pub mod token_refresh;
pub mod refreshing_client;
pub mod response;
pub mod rig_agent;
pub mod system_prompt_facade;
pub mod token_exchange;

/// Default context window for Copilot-hosted models, in tokens.
///
/// PROV-053: This is a deliberately neutral fallback used by the dispatch
/// layer (`ProviderManager::context_window`). The real per-model context
/// window comes from the live `/models` endpoint payload (PROV-056) — fspec
/// itself never hardcodes any model details, including this number for any
/// specific model.
///
/// 200_000 was chosen because it is the value most other providers in this
/// crate also fall back to, so the runtime budget calculations stay
/// consistent across providers when a model has not yet been selected.
pub const CONTEXT_WINDOW: usize = 200_000;

/// Default maximum output tokens for Copilot-hosted models.
///
/// PROV-053: As with [`CONTEXT_WINDOW`], the real per-model value comes from
/// the live `/models` endpoint payload (PROV-056). 4_096 is the smallest
/// value that keeps streaming budgets safe for every Copilot-exposed model
/// and matches the OpenAI provider's default fallback.
pub const MAX_OUTPUT_TOKENS: usize = 4_096;

pub use auth::{
    delete_copilot_auth, get_copilot_auth_path, read_copilot_auth, read_copilot_auth_sync,
    write_copilot_auth, CopilotAuthJson,
};
pub use base_url::{base_url_for, CopilotBaseUrl};
pub use behavior_facade::{
    select_copilot_behavior_facade, BoxedCopilotBehaviorFacade, CopilotBehaviorFacade,
    CopilotClaudeBehaviorFacade, CopilotGeminiBehaviorFacade, CopilotGptBehaviorFacade,
};
pub use classifier::{CopilotRequestClassifier, RequestClassification};
pub use endpoint::{CopilotEndpoint, CopilotEndpointFacade};
pub use header_facade::CopilotHeaderFacade;
pub use model_family::{is_claude_model, is_gemini_model, is_gpt_model};
pub use oauth::copilot_device_auth_login;
pub use oauth_device_code::{normalize_enterprise_domain, request_device_code};
pub use oauth_polling::poll_device_token;
pub use oauth_types::{
    CopilotDeploymentType, CopilotDeviceAuthConfig, CopilotDeviceCodeResponse,
    CopilotDisplayCallback, CopilotPollConfig, CopilotPollResult, COPILOT_CLIENT_ID,
    COPILOT_DEFAULT_HOST, COPILOT_OAUTH_SCOPE,
};
pub use provider::CopilotProvider;
pub use provider_options::apply_store_false;
pub use refreshing_client::CopilotHttpClient;
pub use system_prompt_facade::{
    system_prompt_facade_for_endpoint, CopilotChatCompletionsSystemPromptFacade,
    CopilotResponsesSystemPromptFacade,
};
pub use token_exchange::{
    build_token_exchange_url, exchange_github_token_for_copilot_token,
    exchange_github_token_for_copilot_token_at, TokenExchangeResponse,
};
