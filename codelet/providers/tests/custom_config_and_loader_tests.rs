#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/provider-config-loader-and-rhai-script-compiler.feature
//!
//! Integration tests for PROV-062: custom provider ProviderConfig loader
//! and Rhai ScriptLoader with AST caching.
//!
//! These tests exercise `codelet_providers::custom` which does not yet exist —
//! they must fail to compile in the red phase.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;

#[path = "custom_test_helpers.rs"]
mod helpers;

use helpers::{
    minimal_cfg, write_valid_script, CwdGuard, EnvGuard, SCRIPT_CALLING_PKCE,
    SCRIPT_MISSING_PARSE_RESPONSE, SCRIPT_SYNTAX_ERROR, VALID_SCRIPT,
};

use codelet_providers::custom::error::CustomProviderError;
use codelet_providers::custom::{
    discover_provider_configs, AuthConfig, ProviderConfig, ScriptLoader,
};
use codelet_providers::oauth::building_blocks::register_all_modules;
use codelet_providers::oauth::engine::build_sandboxed_engine;

// =========================================================================
// Scenario: Load a complete custom provider config JSON
// =========================================================================
#[test]
fn load_a_complete_custom_provider_config_json() {
    // @step Given a JSON file containing name, display_name, base_url, script, auth, and models fields
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "my-llm.rhai");
    let cfg = json!({
        "name": "my-llm",
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "MY_KEY" },
        "models": {
            "fast": { "id": "model-fast-v1" },
            "smart": { "id": "model-smart-v2" }
        }
    });
    let cfg_path = tmp.path().join("my-llm.json");
    fs::write(&cfg_path, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();

    // @step When I call ProviderConfig::from_file on that JSON path
    let loaded = ProviderConfig::from_file(&cfg_path).expect("config loads");

    // @step Then I get a ProviderConfig whose fields match the JSON values
    assert_eq!(loaded.name, "my-llm");
    assert_eq!(loaded.display_name, "My LLM");
    assert_eq!(loaded.base_url, "https://api.example.com");
    assert_eq!(loaded.models.len(), 2);
    assert!(loaded.models.contains_key("fast"));
    assert!(loaded.models.contains_key("smart"));
}

// =========================================================================
// Scenario: Reject config JSON missing the required name field
// =========================================================================
#[test]
fn reject_config_json_missing_the_required_name_field() {
    // @step Given a JSON file that omits the name field
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = json!({
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "MY_KEY" },
        "models": { "smart": { "id": "m" } }
    });
    let cfg_path = tmp.path().join("nameless.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then I receive an error whose message mentions the missing name field
    let err = result.expect_err("should fail");
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("name"), "error should mention 'name': {msg}");
}

// NOTE: PROV-085 removed the "Reject provider name that collides with a
// built-in provider" scenario. The positive-path replacements (load claude
// and load codex without NameConflict) live in
// `custom_provider_script_shadowing_tests.rs` alongside the rest of the
// shadowing feature so the 1 feature ↔ 1 test-file invariant holds.

// =========================================================================
// Scenario: Reject provider name with invalid characters
// =========================================================================
#[test]
fn reject_provider_name_with_invalid_characters() {
    // @step Given a config JSON with name set to "My Provider"
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = minimal_cfg(
        "My Provider",
        &script_path.file_name().unwrap().to_string_lossy(),
    );
    let cfg_path = tmp.path().join("bad-name.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then I receive an error mentioning the allowed pattern ^[a-z][a-z0-9-]*$
    let err = result.expect_err("should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("^[a-z][a-z0-9-]*$"),
        "error should contain pattern: {msg}"
    );
}

// =========================================================================
// Scenario: Reject config when referenced script file does not exist
// =========================================================================
#[test]
fn reject_config_when_referenced_script_file_does_not_exist() {
    // @step Given a config JSON whose script field points to a nonexistent .rhai file
    let tmp = TempDir::new().expect("tempdir");
    let cfg = minimal_cfg("my-llm", "./does-not-exist.rhai");
    let cfg_path = tmp.path().join("my-llm.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then I receive an error including the resolved absolute script path
    let err = result.expect_err("should fail");
    assert!(
        matches!(err, CustomProviderError::ScriptNotFound { .. }),
        "expected ScriptNotFound, got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("does-not-exist.rhai"),
        "error should include resolved script path: {msg}"
    );
}

// =========================================================================
// Scenario: Reject config when default model is not present in models map
// =========================================================================
#[test]
fn reject_config_when_default_model_is_not_present_in_models_map() {
    // @step Given a config JSON with defaults.model set to "fast" and models containing only "smart"
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = json!({
        "name": "my-llm",
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "MY_KEY" },
        "models": { "smart": { "id": "m-smart" } },
        "defaults": { "model": "fast" }
    });
    let cfg_path = tmp.path().join("my-llm.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let result = ProviderConfig::from_file(&cfg_path);

    // @step Then I receive an error mentioning the missing default model "fast"
    let err = result.expect_err("should fail");
    assert!(
        matches!(err, CustomProviderError::MissingDefaultModel { .. }),
        "expected MissingDefaultModel, got {err:?}"
    );
    let msg = format!("{err}");
    assert!(msg.contains("fast"), "error should mention 'fast': {msg}");
}

// =========================================================================
// Scenario: Project-local config overrides global config with same name
// =========================================================================
#[test]
#[serial]
fn project_local_config_overrides_global_config_with_same_name() {
    // @step Given a global config ~/.fspec/providers/my-llm.json and a project-local .fspec/providers/my-llm.json both named "my-llm"
    let home_tmp = TempDir::new().expect("home tempdir");
    let project_tmp = TempDir::new().expect("project tempdir");

    // Global: <HOME>/.fspec/providers/my-llm.json. FSPEC_HOME points at
    // <HOME>/.fspec/credentials so discovery resolves the providers dir
    // as its sibling (see research doc section 2.2).
    let fspec_dir = home_tmp.path().join(".fspec");
    let credentials_dir = fspec_dir.join("credentials");
    let global_providers = fspec_dir.join("providers");
    fs::create_dir_all(&credentials_dir).unwrap();
    fs::create_dir_all(&global_providers).unwrap();
    let global_script = write_valid_script(&global_providers, "my-llm.rhai");
    let global_cfg = json!({
        "name": "my-llm",
        "display_name": "Global My LLM",
        "base_url": "https://global.example.com",
        "script": global_script.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "GLOBAL_KEY" },
        "models": { "smart": { "id": "global-smart" } }
    });
    fs::write(
        global_providers.join("my-llm.json"),
        serde_json::to_string(&global_cfg).unwrap(),
    )
    .unwrap();

    // Project-local: <PROJECT>/.fspec/providers/my-llm.json
    let local_providers = project_tmp.path().join(".fspec").join("providers");
    fs::create_dir_all(&local_providers).unwrap();
    let local_script = write_valid_script(&local_providers, "my-llm.rhai");
    let local_cfg = json!({
        "name": "my-llm",
        "display_name": "Local My LLM",
        "base_url": "https://local.example.com",
        "script": local_script.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "LOCAL_KEY" },
        "models": { "smart": { "id": "local-smart" } }
    });
    fs::write(
        local_providers.join("my-llm.json"),
        serde_json::to_string(&local_cfg).unwrap(),
    )
    .unwrap();

    let _home_guard = EnvGuard::set("HOME", home_tmp.path());
    let _fspec_guard = EnvGuard::set("FSPEC_HOME", &credentials_dir);
    let _cwd_guard = CwdGuard::set(project_tmp.path());

    // @step When I call discover_provider_configs
    let configs = discover_provider_configs().expect("discover ok");

    // @step Then the returned list contains exactly one config for "my-llm" and it matches the project-local JSON
    let matching: Vec<&ProviderConfig> =
        configs.iter().filter(|c| c.name == "my-llm").collect();
    assert_eq!(matching.len(), 1, "expected exactly one my-llm config");
    assert_eq!(matching[0].display_name, "Local My LLM");
    assert_eq!(matching[0].base_url, "https://local.example.com");
}

// =========================================================================
// Scenario: Return empty result when no providers directories exist
// =========================================================================
#[test]
#[serial]
fn return_empty_result_when_no_providers_directories_exist() {
    // @step Given neither ~/.fspec/providers/ nor .fspec/providers/ exists
    let home_tmp = TempDir::new().expect("home tempdir");
    let project_tmp = TempDir::new().expect("project tempdir");
    // Do NOT create .fspec/providers anywhere.
    let _home_guard = EnvGuard::set("HOME", home_tmp.path());
    let _fspec_guard = EnvGuard::remove("FSPEC_HOME");
    let _cwd_guard = CwdGuard::set(project_tmp.path());

    // @step When I call discover_provider_configs
    let result = discover_provider_configs().expect("discover ok");

    // @step Then I receive an empty Vec without error
    assert!(result.is_empty(), "expected empty vec, got {result:?}");
}

// =========================================================================
// Scenario: ScriptLoader caches AST for unchanged script
// =========================================================================
#[test]
fn script_loader_caches_ast_for_unchanged_script() {
    // @step Given a valid .rhai file on disk that has not been modified between loads
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "cached.rhai");
    let engine = build_sandboxed_engine(register_all_modules());
    let loader = ScriptLoader::new(engine);

    // @step When I call ScriptLoader::load on the same path twice
    let ast1: Arc<rhai::AST> = loader.load(&script_path).expect("first load");
    let ast2: Arc<rhai::AST> = loader.load(&script_path).expect("second load");

    // @step Then both calls return the same Arc<AST> instance and parsing occurs only once
    assert!(
        Arc::ptr_eq(&ast1, &ast2),
        "expected Arc::ptr_eq for cached AST"
    );
}

// =========================================================================
// Scenario: ScriptLoader re-parses script when mtime changes
// =========================================================================
#[test]
fn script_loader_reparses_script_when_mtime_changes() {
    // @step Given a .rhai file that has been loaded once
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "reload.rhai");
    let engine = build_sandboxed_engine(register_all_modules());
    let loader = ScriptLoader::new(engine);
    let ast1 = loader.load(&script_path).expect("first load");

    // @step When I modify the file so its mtime advances and call ScriptLoader::load again
    std::thread::sleep(Duration::from_millis(1100));
    let updated = format!("{VALID_SCRIPT}\nfn extra() {{ 42 }}\n");
    fs::write(&script_path, updated).expect("rewrite script");
    let ast2 = loader.load(&script_path).expect("second load");

    // @step Then a new Arc<AST> is returned reflecting the updated script content
    assert!(
        !Arc::ptr_eq(&ast1, &ast2),
        "expected different Arc<AST> after mtime change"
    );
    let has_extra = ast2.iter_functions().any(|f| f.name == "extra");
    assert!(
        has_extra,
        "reparsed AST should contain new function 'extra'"
    );
}

// =========================================================================
// Scenario: Report Rhai syntax errors with file path line and column
// =========================================================================
#[test]
fn report_rhai_syntax_errors_with_file_path_line_and_column() {
    // @step Given a .rhai file containing a syntactically invalid function declaration
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("bad-syntax.rhai");
    fs::write(&script_path, SCRIPT_SYNTAX_ERROR).expect("write bad script");
    let engine = build_sandboxed_engine(register_all_modules());
    let loader = ScriptLoader::new(engine);

    // @step When I call ScriptLoader::load on that file
    let result = loader.load(&script_path);

    // @step Then the returned error includes the file path and the line and column from the Rhai ParseError
    let err = result.expect_err("syntax error expected");
    assert!(
        matches!(err, CustomProviderError::RhaiParseError { .. }),
        "expected RhaiParseError, got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("bad-syntax.rhai"),
        "error should include file path: {msg}"
    );
    assert!(
        msg.contains("line") || msg.contains(':'),
        "error should include line/column info: {msg}"
    );
}

// =========================================================================
// Scenario: Reject script missing a required function
// =========================================================================
#[test]
fn reject_script_missing_a_required_function() {
    // @step Given a .rhai file that parses but does not define parse_response
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("missing-fn.rhai");
    fs::write(&script_path, SCRIPT_MISSING_PARSE_RESPONSE).expect("write script");
    let engine = build_sandboxed_engine(register_all_modules());
    let loader = ScriptLoader::new(engine);
    let ast = loader.load(&script_path).expect("script parses");

    // @step When I validate the compiled script against the required functions list
    let result = loader.validate_required_functions(&ast);

    // @step Then I receive an error naming parse_response as the missing function
    let err = result.expect_err("validation should fail");
    assert!(
        matches!(err, CustomProviderError::MissingFunction { .. }),
        "expected MissingFunction, got {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("parse_response"),
        "error should mention parse_response: {msg}"
    );
}

// =========================================================================
// Scenario: Compiled script can call registered PROV-060 building blocks
// =========================================================================
#[test]
fn compiled_script_can_call_registered_prov060_building_blocks() {
    // @step Given a .rhai file that calls oauth::generate_pkce inside a function
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("pkce.rhai");
    fs::write(&script_path, SCRIPT_CALLING_PKCE).expect("write script");

    // @step When I compile it with the shared sandboxed engine and execute that function
    let engine = build_sandboxed_engine(register_all_modules());
    let loader = ScriptLoader::new(engine);
    let ast = loader.load(&script_path).expect("script compiles");

    let mut scope = rhai::Scope::new();
    let result: rhai::Map = loader
        .engine()
        .call_fn(&mut scope, &ast, "make_pkce", ())
        .expect("call_fn ok");

    // @step Then the script runs successfully and returns a PKCE pair
    assert!(
        result.contains_key("verifier"),
        "pkce pair should contain 'verifier' key"
    );
    assert!(
        result.contains_key("challenge"),
        "pkce pair should contain 'challenge' key"
    );
}

// =========================================================================
// Scenario: Bearer auth config deserializes with default token prefix
// =========================================================================
#[test]
fn bearer_auth_config_deserializes_with_default_token_prefix() {
    // @step Given a config JSON with auth.type set to "bearer" and auth.env_var set to "MY_KEY"
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = json!({
        "name": "my-llm",
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "MY_KEY" },
        "models": { "smart": { "id": "m" } }
    });
    let cfg_path = tmp.path().join("bearer.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let loaded = ProviderConfig::from_file(&cfg_path).expect("config loads");

    // @step Then the auth field is AuthConfig::Bearer with env_var "MY_KEY" and token_prefix "Bearer"
    match loaded.auth {
        AuthConfig::Bearer {
            env_var,
            token_prefix,
        } => {
            assert_eq!(env_var, "MY_KEY");
            assert_eq!(token_prefix, "Bearer");
        }
        other => panic!("expected AuthConfig::Bearer, got {other:?}"),
    }
}

// =========================================================================
// Scenario: OAuth device code auth config deserializes with all fields
// =========================================================================
#[test]
fn oauth_device_code_auth_config_deserializes_with_all_fields() {
    // @step Given a config JSON with auth.type set to "oauth_device_code" and client_id, device_code_url, token_url, credential_file all provided
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_valid_script(tmp.path(), "p.rhai");
    let cfg = json!({
        "name": "my-llm",
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": {
            "type": "oauth_device_code",
            "client_id": "client-xyz",
            "device_code_url": "https://auth.example.com/device",
            "token_url": "https://auth.example.com/token",
            "credential_file": "my_provider_auth.json"
        },
        "models": { "smart": { "id": "m" } }
    });
    let cfg_path = tmp.path().join("oauth-dc.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    // @step When I load the config
    let loaded = ProviderConfig::from_file(&cfg_path).expect("config loads");

    // @step Then the auth field is AuthConfig::OauthDeviceCode with matching fields
    match loaded.auth {
        AuthConfig::OauthDeviceCode {
            client_id,
            device_code_url,
            token_url,
            credential_file,
            ..
        } => {
            assert_eq!(client_id, "client-xyz");
            assert_eq!(device_code_url, "https://auth.example.com/device");
            assert_eq!(token_url, "https://auth.example.com/token");
            assert_eq!(credential_file, "my_provider_auth.json");
        }
        other => panic!("expected AuthConfig::OauthDeviceCode, got {other:?}"),
    }
}
