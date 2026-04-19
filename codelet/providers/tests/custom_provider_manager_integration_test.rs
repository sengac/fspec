#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-manager-integration.feature
//!
//! PROV-067 integration tests — custom providers are wired through
//! [`codelet_providers::ProviderManager`] via a new
//! [`codelet_providers::ProviderType::Custom`] variant, facade_override
//! dispatch, and credential discovery from
//! `~/.fspec/providers/*.json` + `.fspec/providers/*.json`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;

use codelet_providers::custom::{
    discover_provider_configs, init_provider_template, list_providers_info,
    show_provider_info, test_provider_connection, validate_provider_config,
    CustomProvider, ProviderConfig,
};
use codelet_providers::{ProviderCredentials, ProviderManager, ProviderType};

use std::str::FromStr;

// =========================================================================
// Shared helpers
// =========================================================================

/// RAII env-var guard that restores the previous value on drop.
struct EnvGuard {
    key: String,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            prior,
        }
    }

    fn remove(key: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::remove_var(key);
        Self {
            key: key.to_string(),
            prior,
        }
    }

    fn set_path(key: &str, value: &Path) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            prior,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prior.take() {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

/// RAII CWD guard.
struct CwdGuard {
    prior: PathBuf,
}

impl CwdGuard {
    fn set(new_cwd: &Path) -> Self {
        let prior = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(new_cwd).expect("set cwd");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// Per-test combined fixture that redirects HOME + FSPEC_HOME + CWD at
/// fresh temp directories so discovery never reads the user's real
/// `~/.fspec/providers/`.
struct DiscoveryFixture {
    _home_tmp: TempDir,
    project_tmp: TempDir,
    _home_guard: EnvGuard,
    _fspec_guard: EnvGuard,
    _cwd_guard: CwdGuard,
}

impl DiscoveryFixture {
    fn new() -> Self {
        let home_tmp = TempDir::new().expect("home tempdir");
        let project_tmp = TempDir::new().expect("project tempdir");
        let fspec_dir = home_tmp.path().join(".fspec");
        let credentials_dir = fspec_dir.join("credentials");
        fs::create_dir_all(&credentials_dir).unwrap();
        let home_guard = EnvGuard::set_path("HOME", home_tmp.path());
        let fspec_guard = EnvGuard::set_path("FSPEC_HOME", &credentials_dir);
        let cwd_guard = CwdGuard::set(project_tmp.path());
        Self {
            _home_tmp: home_tmp,
            project_tmp,
            _home_guard: home_guard,
            _fspec_guard: fspec_guard,
            _cwd_guard: cwd_guard,
        }
    }

    fn project_root(&self) -> &Path {
        self.project_tmp.path()
    }
}

/// Write a project-local custom provider config under
/// `<project>/.fspec/providers/<name>.json` using the facade-mode schema
/// (no script, optional `api_key_env_var`, optional `facade`).
fn write_project_custom_provider(
    project_root: &Path,
    name: &str,
    facade: Option<&str>,
    base_url: &str,
    api_key_env_var: Option<&str>,
) {
    let providers_dir = project_root.join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).unwrap();
    let mut obj = json!({
        "name": name,
        "display_name": format!("Test {}", name),
        "base_url": base_url,
        "models": {
            "llama-3.1-70b": { "id": "llama-3.1-70b" },
            "qwen-2.5-coder-32b": { "id": "qwen-2.5-coder-32b" }
        }
    });
    if let Some(f) = facade {
        obj.as_object_mut()
            .unwrap()
            .insert("facade".into(), json!(f));
    }
    if let Some(env) = api_key_env_var {
        obj.as_object_mut()
            .unwrap()
            .insert("api_key_env_var".into(), json!(env));
    }
    fs::write(
        providers_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&obj).unwrap(),
    )
    .unwrap();
}

// =========================================================================
// Scenario: Initialize custom provider definition from openai-compatible template
// =========================================================================
#[test]
#[serial]
fn initialize_custom_provider_definition_from_openai_compatible_template() {
    // @step Given I have a project root with no .fspec/providers/ directory
    let fx = DiscoveryFixture::new();
    let providers_dir = fx.project_root().join(".fspec").join("providers");
    assert!(!providers_dir.exists(), "precondition: providers dir absent");

    // @step When I run 'codelet providers init my-llm --template openai-compatible'
    let written = init_provider_template(fx.project_root(), "my-llm", "openai-compatible")
        .expect("init should succeed");

    // @step Then the file .fspec/providers/my-llm.json is created with name=my-llm and facade=openai
    assert_eq!(written, providers_dir.join("my-llm.json"));
    let raw = fs::read_to_string(&written).expect("file readable");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("json parses");
    assert_eq!(parsed.get("name").and_then(|v| v.as_str()), Some("my-llm"));
    assert_eq!(
        parsed.get("facade").and_then(|v| v.as_str()),
        Some("openai"),
    );

    // @step And the file contains placeholder baseUrl and apiKeyEnvVar fields
    assert!(
        parsed.get("base_url").and_then(|v| v.as_str()).is_some(),
        "template must carry a base_url placeholder"
    );
    assert!(
        parsed
            .get("api_key_env_var")
            .and_then(|v| v.as_str())
            .is_some(),
        "template must carry an api_key_env_var placeholder"
    );
}

// =========================================================================
// Scenario: List providers shows custom providers with credential status
// =========================================================================
#[test]
#[serial]
fn list_providers_shows_custom_providers_with_credential_status() {
    // @step Given a project with .fspec/providers/my-llm.json defining name=my-llm apiKeyEnvVar=MY_LLM_API_KEY
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );
    // @step And the environment variable MY_LLM_API_KEY is set to a non-empty value
    let _key_guard = EnvGuard::set("MY_LLM_API_KEY", "sk-not-real");

    // @step When I call list_providers()
    let providers = list_providers_info().expect("list ok");

    // @step Then the result includes an entry with name='my-llm', isCustom=true, available=true
    let my_entry = providers
        .iter()
        .find(|p| p.name == "my-llm")
        .expect("my-llm present in list");
    assert!(my_entry.is_custom, "my-llm should be marked custom");
    assert!(
        my_entry.available,
        "my-llm should be available when MY_LLM_API_KEY is set"
    );

    // @step And the result also includes built-in providers like claude and openai
    let names: Vec<&str> = providers.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"claude"));
    assert!(names.contains(&"openai"));
}

// =========================================================================
// Scenario: Show custom provider returns full definition
// =========================================================================
#[test]
#[serial]
fn show_custom_provider_returns_full_definition() {
    // @step Given a custom provider 'my-llm' is discovered with facade=openai, baseUrl=http://localhost:8888/v1, 2 models, apiKeyEnvVar=MY_LLM_API_KEY
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );

    // @step When I call show_provider('my-llm')
    let info = show_provider_info("my-llm").expect("show ok");

    // @step Then the returned info includes name, displayName, facade, baseUrl, apiKeyEnvVar, and the 2 models
    assert_eq!(info.name, "my-llm");
    assert_eq!(info.display_name.as_deref(), Some("Test my-llm"));
    assert_eq!(info.facade.as_deref(), Some("openai"));
    assert_eq!(info.base_url.as_deref(), Some("http://localhost:8888/v1"));
    assert_eq!(info.api_key_env_var.as_deref(), Some("MY_LLM_API_KEY"));
    assert_eq!(info.models.len(), 2, "should report 2 models");
}

// =========================================================================
// Scenario: Validate custom provider reports schema violations
// =========================================================================
#[test]
#[serial]
fn validate_custom_provider_reports_schema_violations() {
    // @step Given a file .fspec/providers/broken.json missing required field 'facade'
    let fx = DiscoveryFixture::new();
    let providers_dir = fx.project_root().join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).unwrap();
    // A config that omits both facade AND the script path — by design
    // validate_provider_config flags missing facade when no script is
    // present (because facade=null requires a script).
    fs::write(
        providers_dir.join("broken.json"),
        json!({
            "name": "broken",
            "display_name": "Broken",
            "base_url": "http://example.com",
            "models": { "m": { "id": "m" } }
        })
        .to_string(),
    )
    .unwrap();

    // @step When I call validate_provider('broken')
    let result = validate_provider_config("broken");

    // @step Then the result is an error describing the missing 'facade' field
    let err = result.expect_err("should fail validation");
    let msg = err.to_string();
    assert!(
        msg.contains("facade"),
        "error should mention the missing 'facade' field, got: {msg}"
    );
}

// =========================================================================
// Scenario: Test custom provider performs connectivity check against baseUrl
// =========================================================================
#[tokio::test]
#[serial]
async fn test_custom_provider_performs_connectivity_check_against_base_url() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // @step Given a custom provider 'my-llm' with baseUrl pointing to a mock HTTP server returning 200 and a /v1/models response listing 'llama-3.1-70b'
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                { "id": "llama-3.1-70b", "object": "model" }
            ]
        })))
        .mount(&server)
        .await;

    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        &format!("{}/v1", server.uri()),
        Some("MY_LLM_API_KEY"),
    );
    let _key_guard = EnvGuard::set("MY_LLM_API_KEY", "sk-test");

    // @step When I call test_provider('my-llm')
    let result = test_provider_connection("my-llm").await.expect("probe ok");

    // @step Then the result is Ok with reachable=true and at least one model matched
    assert!(result.reachable, "baseUrl must be reachable");
    assert!(
        result.matched_models.iter().any(|m| m == "llama-3.1-70b"),
        "expected llama-3.1-70b among matched models, got: {:?}",
        result.matched_models
    );
}

// =========================================================================
// Scenario: Select custom model routes through openai facade via facade_override
// =========================================================================
#[test]
#[serial]
fn select_custom_model_routes_through_openai_facade_via_facade_override() {
    // @step Given a ProviderManager with custom provider 'my-llm' discovered (facade=openai, baseUrl=http://localhost:8888/v1)
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );
    let _key_guard = EnvGuard::set("MY_LLM_API_KEY", "sk-test");
    let _baseurl_guard = EnvGuard::remove("OPENAI_BASE_URL");

    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);

    // @step When I call set_model_direct('my-llm', 'llama-3.1-70b', Some(131072), Some(4096), Some('openai'))
    manager
        .set_model_direct(
            "my-llm",
            "llama-3.1-70b",
            Some(131_072),
            Some(4096),
            Some("openai".to_string()),
        )
        .expect("set_model_direct should succeed for custom provider");

    // @step Then current_provider equals ProviderType::Custom("my-llm")
    assert_eq!(
        manager.current_provider_type().clone(),
        ProviderType::Custom("my-llm".to_string())
    );

    // @step And facade_override returns Some("openai")
    assert_eq!(manager.facade_override(), Some("openai"));

    // @step And OPENAI_BASE_URL environment variable equals 'http://localhost:8888/v1'
    // (This is set by `apply_custom_provider_env_vars` which the NAPI
    // layer invokes after set_model_direct; we call it directly here.)
    codelet_providers::custom::apply_custom_provider_env_vars(
        "my-llm",
        "llama-3.1-70b",
        Some("openai"),
    )
    .expect("env var propagation should succeed");
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some("http://localhost:8888/v1"),
    );
    // Clean up the env var we just set so other tests don't inherit it.
    std::env::remove_var("OPENAI_BASE_URL");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_MODEL");
}

// =========================================================================
// Scenario: Agent loop dispatches custom provider via facade_override to existing match arm
// =========================================================================
#[test]
#[serial]
fn agent_loop_dispatches_custom_provider_via_facade_override_to_existing_match_arm() {
    // @step Given a ProviderManager with current_provider=Custom("my-llm") and facade_override=Some("openai")
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );
    // Provide the credential the 'openai' facade arm would forward into
    // OPENAI_API_KEY; keep every OPENAI_* guard scoped so other tests do
    // not inherit the mutation.
    let _key_guard = EnvGuard::set("MY_LLM_API_KEY", "sk-test");
    let _baseurl_guard = EnvGuard::remove("OPENAI_BASE_URL");
    let _apikey_guard = EnvGuard::remove("OPENAI_API_KEY");
    let _model_guard = EnvGuard::remove("OPENAI_MODEL");
    let mut manager = ProviderManager::for_testing(
        ProviderType::Custom("my-llm".to_string()),
        None,
        None,
    );
    manager.set_facade_override(Some("openai".to_string()));

    // @step And OPENAI_BASE_URL has been applied from the custom provider via apply_custom_provider_env_vars
    codelet_providers::custom::apply_custom_provider_env_vars(
        "my-llm",
        "llama-3.1-70b",
        Some("openai"),
    )
    .expect("env var propagation should succeed");

    // @step When the agent loop resolves the dispatch string via facade_override().unwrap_or(current_provider_name())
    let dispatch = manager
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| manager.current_provider_name().to_string());

    // @step Then the resolved dispatch string equals 'openai'
    assert_eq!(dispatch, "openai");

    // @step And the current provider type remains ProviderType::Custom("my-llm")
    assert_eq!(
        manager.current_provider_type().clone(),
        ProviderType::Custom("my-llm".to_string()),
    );

    // @step And OPENAI_BASE_URL reflects the custom provider's base_url so the 'openai' match arm picks up the custom endpoint transparently
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some("http://localhost:8888/v1"),
        "OPENAI_BASE_URL must match the custom provider's base_url so the agent-loop 'openai' arm hits the custom endpoint"
    );
    assert_eq!(
        std::env::var("OPENAI_MODEL").ok().as_deref(),
        Some("llama-3.1-70b"),
        "OPENAI_MODEL must match the model routed through the facade",
    );
    assert_eq!(
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        Some("sk-test"),
        "OPENAI_API_KEY must be forwarded from the custom provider's apiKeyEnvVar",
    );
}

// =========================================================================
// Scenario: Custom provider is unavailable when required env var is unset
// =========================================================================
#[test]
#[serial]
fn custom_provider_is_unavailable_when_required_env_var_is_unset() {
    // @step Given a custom provider 'my-llm' with apiKeyEnvVar=MY_LLM_API_KEY is discovered
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );
    // @step And the environment variable MY_LLM_API_KEY is not set
    let _env_guard = EnvGuard::remove("MY_LLM_API_KEY");

    // @step When I call ProviderCredentials::detect()
    let credentials = ProviderCredentials::detect();

    // @step Then credentials.has_custom("my-llm") returns false
    assert!(!credentials.has_custom("my-llm"));

    // @step And ProviderType::Custom("my-llm").has_credentials(&credentials) returns false
    let ptype = ProviderType::Custom("my-llm".to_string());
    assert!(!ptype.has_credentials(&credentials));
}

// =========================================================================
// Scenario: Project-local custom provider definition overrides user-global
// =========================================================================
#[test]
#[serial]
fn project_local_custom_provider_definition_overrides_user_global() {
    // @step Given a user-global definition at ~/.fspec/providers/my-llm.json with baseUrl=http://global/v1
    let fx = DiscoveryFixture::new();
    let global_dir = fx
        ._home_tmp
        .path()
        .join(".fspec")
        .join("providers");
    fs::create_dir_all(&global_dir).unwrap();
    fs::write(
        global_dir.join("my-llm.json"),
        json!({
            "name": "my-llm",
            "display_name": "Global My LLM",
            "base_url": "http://global/v1",
            "facade": "openai",
            "api_key_env_var": "MY_LLM_API_KEY",
            "models": { "llama": { "id": "llama" } }
        })
        .to_string(),
    )
    .unwrap();

    // @step And a project-local definition at <project>/.fspec/providers/my-llm.json with baseUrl=http://local/v1
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://local/v1",
        Some("MY_LLM_API_KEY"),
    );

    // @step When I call discover_custom_providers(Some(project_root))
    let configs = discover_provider_configs().expect("discover ok");
    let my_llm: Vec<&ProviderConfig> = configs.iter().filter(|c| c.name == "my-llm").collect();

    // @step Then the returned map contains exactly one entry 'my-llm' with baseUrl=http://local/v1
    assert_eq!(my_llm.len(), 1, "expected exactly one my-llm entry");
    assert_eq!(my_llm[0].base_url, "http://local/v1");
}

// =========================================================================
// Scenario: Custom provider without facade uses generic CustomProvider create_rig_agent
// =========================================================================
#[test]
#[serial]
fn custom_provider_without_facade_uses_generic_custom_provider_create_rig_agent() {
    // @step Given a custom provider 'rhai-llm' discovered with facade=null and a Rhai script defining define_tools and format_system_prompt
    let fx = DiscoveryFixture::new();
    let providers_dir = fx.project_root().join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).unwrap();
    let script_path = providers_dir.join("rhai-llm.rhai");
    fs::write(
        &script_path,
        r#"
fn build_request(ctx) { #{} }
fn build_headers(ctx) { #{} }
fn build_url(ctx) { "http://example.com" }
fn parse_response(resp) { #{ content: #{ type: "text", text: "" }, stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(err) { #{} }
fn define_tools(config) {
    [
        #{
            name: "read_file",
            description: "Read a file",
            parameters: #{ type: "object" },
            maps_to: "file:read",
        },
    ]
}
fn format_system_prompt(config, preamble, fspec_guidance) { preamble }
"#,
    )
    .unwrap();
    fs::write(
        providers_dir.join("rhai-llm.json"),
        json!({
            "name": "rhai-llm",
            "display_name": "Rhai LLM",
            "base_url": "http://example.com",
            "script": "rhai-llm.rhai",
            "auth": { "type": "bearer", "env_var": "RHAI_LLM_KEY" },
            "models": { "default": { "id": "default" } }
        })
        .to_string(),
    )
    .unwrap();

    // @step When the agent loop requests an agent for 'rhai-llm' with no facade_override
    let session_id = uuid::Uuid::new_v4();
    let agent_result = CustomProvider::create_rig_agent(
        fx.project_root(),
        "rhai-llm",
        "default",
        session_id,
        None,
    );

    // @step Then CustomProvider::create_rig_agent is invoked and wires RhaiToolFacadeAdapter instances from the script's define_tools output
    // @step And the agent uses RhaiSystemPromptFacade to format the system prompt
    let agent = agent_result.expect("create_rig_agent should succeed");
    // The agent should advertise RhaiSystemPromptFacade-provided prefix
    // (None in this script) and RhaiToolFacadeAdapter as the tool facade.
    assert_eq!(agent.provider_name(), "rhai-llm");
    assert!(agent.uses_rhai_system_prompt_facade(), "must use RhaiSystemPromptFacade");
    assert!(
        agent.uses_rhai_tool_facade_adapter(),
        "must wire RhaiToolFacadeAdapter"
    );
}

// =========================================================================
// Scenario: FromStr resolves registered custom provider slug to ProviderType::Custom
// =========================================================================
#[test]
#[serial]
fn from_str_resolves_registered_custom_provider_slug_to_provider_type_custom() {
    // @step Given a custom provider 'my-llm' is discovered and registered
    let fx = DiscoveryFixture::new();
    write_project_custom_provider(
        fx.project_root(),
        "my-llm",
        Some("openai"),
        "http://localhost:8888/v1",
        Some("MY_LLM_API_KEY"),
    );

    // @step When I call ProviderType::from_str("my-llm")
    let result = ProviderType::from_str("my-llm");

    // @step Then the result is Ok(ProviderType::Custom("my-llm"))
    assert_eq!(
        result.expect("from_str should succeed"),
        ProviderType::Custom("my-llm".to_string())
    );

    // @step And ProviderType::from_str("nonexistent") returns a config error
    let err = ProviderType::from_str("nonexistent-provider-xyz")
        .expect_err("nonexistent should error");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("unknown")
            || msg.to_lowercase().contains("configuration"),
        "error should indicate unknown/config error, got: {msg}"
    );
}

// =========================================================================
// Scenario: Detect default provider never auto-selects a custom provider
// =========================================================================
#[test]
#[serial]
fn detect_default_provider_never_auto_selects_a_custom_provider() {
    // @step Given ProviderCredentials with all built-in providers unavailable
    // @step And custom_available contains 'my-llm' set to true
    let mut custom = HashMap::new();
    custom.insert("my-llm".to_string(), true);
    let credentials = ProviderCredentials {
        claude_available: false,
        openai_available: false,
        codex_available: false,
        gemini_available: false,
        zai_available: false,
        github_copilot_available: false,
        custom_available: custom,
    };
    assert!(
        credentials.has_custom("my-llm"),
        "precondition: custom_available should contain 'my-llm'=true"
    );

    // @step When I call ProviderManager::detect_default_provider(&credentials)
    let result = ProviderManager::detect_default_provider_for_test(&credentials);

    // @step Then the result is an auth error with message 'No provider credentials available'
    let err = result.expect_err("detect should fail when only custom is available");
    assert!(
        err.to_string().contains("No provider credentials available"),
        "error should mention 'No provider credentials available', got: {err}"
    );
}
