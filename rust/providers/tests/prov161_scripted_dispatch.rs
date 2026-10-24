//! Feature: spec/features/provider-runtime-scripted-dispatch.feature

use codelet_providers::custom::{CustomProvider, CustomRigAgent};
use codelet_providers::ProviderManager;
use codelet_test_helpers::setup_temp_directory;
use rig::completion::Prompt;
use serde_json::{json, Value};
use serial_test::serial;
use std::ffi::OsString;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult<Output = ()> = Result<Output, Box<dyn std::error::Error>>;

struct Fixture {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
    listener: TcpListener,
}

impl Fixture {
    async fn new(models: Value) -> TestResult<Self> {
        let directory = setup_temp_directory()?;
        let mut fixture = Self {
            directory,
            environment: Vec::new(),
            listener: TcpListener::bind("127.0.0.1:0").await?,
        };
        for (name, value) in [
            (
                "FSPEC_USER_DIR",
                fixture.directory.path().as_os_str().to_owned(),
            ),
            (
                "FSPEC_HOME",
                fixture
                    .directory
                    .path()
                    .join("credentials")
                    .into_os_string(),
            ),
            ("OPENAI_API_KEY", OsString::from("offline-dispatch-key")),
            ("FSPEC_DISABLE_SCRIPT_SHADOWING", OsString::from("1")),
        ] {
            fixture.environment.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        let providers = fixture.directory.path().join("providers");
        std::fs::create_dir_all(&providers)?;
        std::fs::write(providers.join("scripted.rhai"), SCRIPT)?;
        std::fs::write(
            providers.join("scripted.json"),
            serde_json::to_vec(&json!({
                "name":"scripted", "display_name":"Scripted dispatch",
                "base_url":format!("http://{}", fixture.listener.local_addr()?),
                "script":"scripted.rhai", "tool_style":"openai", "models":models
            }))?,
        )?;
        std::fs::write(
            fixture.directory.path().join("fspec-config.json"),
            serde_json::to_vec(&json!({"providers":{"scripted":{
                "profiles":{"careful":{"autoContinue":5}}
            }}}))?,
        )?;
        Ok(fixture)
    }

    fn select(&self, profile: Option<&str>) -> TestResult<ProviderManager> {
        let mut manager = ProviderManager::with_provider("openai")?;
        manager.set_model_direct_with_profile("scripted", "smart", profile, None, None, None)?;
        Ok(manager)
    }

    fn agent(&self, selection: &str) -> TestResult<CustomRigAgent> {
        Ok(CustomProvider::create_rig_agent(
            self.directory.path(),
            "scripted",
            selection,
            uuid::Uuid::new_v4(),
            Some("Verify scripted dispatch"),
            None,
            false,
        )?)
    }

    async fn request(&self, agent: CustomRigAgent) -> TestResult<String> {
        let exchange = async {
            let agent = agent.into_inner();
            let (answer, request) = tokio::join!(agent.prompt("Verify dispatch"), self.receive());
            assert_eq!(answer?, "verified");
            request
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), exchange).await?
    }

    async fn receive(&self) -> TestResult<String> {
        let (mut socket, _) = self.listener.accept().await?;
        let mut data = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let count = socket.read(&mut buffer).await?;
            if count == 0 {
                return Err("HTTP request ended before its complete body".into());
            }
            data.extend_from_slice(&buffer[..count]);
            let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = std::str::from_utf8(&data[..end])?;
            let length = headers.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            });
            if data.len() >= end + 4 + length.ok_or("missing HTTP content length")? {
                break;
            }
        }
        let body = r#"{"content":"verified"}"#;
        socket.write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ).as_bytes()).await?;
        Ok(String::from_utf8(data)?)
    }

    async fn assert_no_request(&self) {
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(25), self.listener.accept())
                .await
                .is_err()
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (name, value) in self.environment.iter().rev() {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

const SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_stream_request(request) { #{} }
fn build_headers(config) {
    #{ "x-model-id": config.model, "x-model-alias": config.model_alias }
}
fn build_url(config) { config.base_url + "/" + config.model }
fn parse_response(raw) { #{ content: raw.content, stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn map_error(status, body) { #{ type: "api", message: body } }
"#;

fn assert_identity(request: &str) {
    assert!(request.starts_with("POST /private/script-model HTTP/1.1\r\n"));
    let headers = request.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-model-id: private/script-model\r\n"));
    assert!(headers.contains("\r\nx-model-alias: smart\r\n"));
}

#[tokio::test]
#[serial]
async fn scenario_selected_scripted_model_dispatches_with_its_original_alias() -> TestResult {
    // @step Given a scripted provider maps smart to upstream model private/script-model
    let fixture = Fixture::new(json!({"smart":{"id":"private/script-model"}})).await?;
    let manager = fixture.select(None)?;
    let selected = manager.selected_model_id().ok_or("missing selected ID")?;
    assert_eq!(selected, "private/script-model");
    assert_eq!(manager.facade_override(), None);
    // @step When the selected model is used to create an agent and send a request
    let request = fixture.request(fixture.agent(&selected)?).await?;
    // @step Then the request uses private/script-model and the script receives alias smart
    assert_identity(&request);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_named_scripted_runtime_profile_preserves_dispatch_identity() -> TestResult {
    // @step Given a scripted provider profile careful selects smart mapped to private/script-model
    let fixture = Fixture::new(json!({"smart":{"id":"private/script-model"}})).await?;
    let manager = fixture.select(Some("careful"))?;
    let selected = manager.selected_model_id().ok_or("missing selected ID")?;
    assert_eq!(selected, "private/script-model");
    // @step When the profile selection is used to create an agent and send a request
    let request = fixture.request(fixture.agent(&selected)?).await?;
    // @step Then the profile remains selected and the request preserves the upstream ID and original alias
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("scripted:careful/smart")
    );
    assert_eq!(manager.runtime_options().auto_continue, Some(5));
    assert_identity(&request);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_exact_scripted_aliases_take_priority_over_upstream_id_matches() -> TestResult {
    // @step Given a scripted model alias smart also matches another model's upstream ID
    let fixture = Fixture::new(json!({
        "smart":{"id":"private/script-model"}, "other":{"id":"smart"}
    }))
    .await?;
    // @step When an agent is created with the exact alias smart and sends a request
    let request = fixture.request(fixture.agent("smart")?).await?;
    // @step Then the request uses the model configured under smart rather than the other alias
    assert_identity(&request);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_unknown_scripted_model_identifiers_are_rejected() -> TestResult {
    // @step Given a scripted provider has no alias or upstream ID matching missing/model
    let fixture = Fixture::new(json!({"smart":{"id":"private/script-model"}})).await?;
    // @step When an agent is created with missing/model
    let result = fixture.agent("missing/model");
    // @step Then construction fails without dispatching a request
    assert!(result.is_err());
    assert!(result
        .err()
        .ok_or("missing unknown-ID error")?
        .to_string()
        .contains("missing/model"));
    fixture.assert_no_request().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_ambiguous_scripted_upstream_ids_are_rejected() -> TestResult {
    // @step Given scripted aliases first and second share upstream ID shared/model
    let fixture = Fixture::new(json!({
        "first":{"id":"shared/model"}, "second":{"id":"shared/model"}
    }))
    .await?;
    // @step When an agent is created with shared/model
    let result = fixture.agent("shared/model");
    // @step Then construction fails without choosing either alias or dispatching a request
    assert!(result.is_err());
    assert!(result
        .err()
        .ok_or("missing ambiguous-ID error")?
        .to_string()
        .contains("shared/model"));
    fixture.assert_no_request().await;
    Ok(())
}
