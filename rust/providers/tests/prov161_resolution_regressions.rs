use codelet_providers::ProviderManager;
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;
use serial_test::serial;
use std::ffi::OsString;

type TestResult<Output = ()> = Result<Output, Box<dyn std::error::Error>>;

struct Environment(Vec<(&'static str, Option<OsString>)>);

impl Environment {
    fn set(&mut self, name: &'static str, value: impl AsRef<std::ffi::OsStr>) {
        self.0.push((name, std::env::var_os(name)));
        std::env::set_var(name, value);
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, value) in self.0.iter().rev() {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

fn configuration() -> TestResult<(tempfile::TempDir, Environment)> {
    let directory = setup_temp_directory()?;
    let mut environment = Environment(Vec::new());
    environment.set("FSPEC_USER_DIR", directory.path());
    environment.set("FSPEC_HOME", directory.path().join("credentials"));
    environment.set("OPENAI_API_KEY", "prov161-standalone");
    environment.set("FSPEC_DISABLE_SCRIPT_SHADOWING", "1");
    std::fs::write(
        directory.path().join("fspec-config.json"),
        serde_json::to_vec(&json!({"providers":{
            "openai":{"settings":{"autoContinue":7}},
            "scripted":{"settings":{"autoContinue":3}}
        }}))?,
    )?;
    Ok((directory, environment))
}

fn scripted_manager() -> TestResult<(tempfile::TempDir, Environment, ProviderManager)> {
    let (directory, environment) = configuration()?;
    let mut manager = ProviderManager::with_provider("openai")?;
    let providers = directory.path().join("providers");
    std::fs::create_dir_all(&providers)?;
    std::fs::write(
        providers.join("scripted.rhai"),
        r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_stream_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { config.base_url }
fn parse_response(raw) { #{ content: "done", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn map_error(status, body) { #{ type: "api", message: body } }
fn get_model_limits(config) {
    #{ context_window: 320000, max_output_tokens: 24000,
       compaction_threshold: #{ type: "percentage", value: 65 } }
}
"#,
    )?;
    std::fs::write(
        providers.join("scripted.json"),
        serde_json::to_vec(&json!({
            "name":"scripted", "display_name":"Scripted", "base_url":"https://scripted.invalid",
            "script":providers.join("scripted.rhai"), "api_style":"anthropic_messages",
            "models":{"smart":{"id":"private/script-model", "context_window":120000,
                "max_output_tokens":12000}}
        }))?,
    )?;
    manager.set_model_direct("scripted", "smart", None, None, None)?;
    Ok((directory, environment, manager))
}

#[test]
#[serial]
fn standalone_constructor_loads_explicit_user_directory() -> TestResult {
    assert!(codelet_common::get_data_dir().is_err());
    let (_directory, _environment) = configuration()?;
    let manager = ProviderManager::with_provider("openai")?;
    assert_eq!(manager.runtime_options().auto_continue, Some(7));
    Ok(())
}

#[test]
#[serial]
fn scripted_selection_preserves_native_identity() -> TestResult {
    let (_directory, _environment, manager) = scripted_manager()?;
    assert_eq!(manager.facade_override(), None);
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("private/script-model")
    );
    assert_eq!(manager.current_provider_name(), "scripted");
    Ok(())
}

#[test]
#[serial]
fn scripted_selection_retains_script_supplied_limits() -> TestResult {
    let (_directory, _environment, manager) = scripted_manager()?;
    assert_eq!(manager.context_window(), 320_000);
    assert_eq!(manager.max_output_tokens(), 24_000);
    assert_eq!(
        manager.compaction_threshold_override(),
        Some(("percentage", 65))
    );
    assert_eq!(manager.runtime_options().auto_continue, Some(3));
    Ok(())
}
