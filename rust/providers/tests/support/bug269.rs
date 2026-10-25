use super::environment::TestResult;
use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_providers::LlmProvider;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub const NAME: &str = "synthetic-request-refresh";
pub const OLD: &str = "synthetic-old-access-secret";
pub const NEW: &str = "synthetic-new-access-secret";
pub const REFRESH: &str = "synthetic-refresh-secret";

pub fn token_path() -> PathBuf {
    codelet_providers::oauth::custom_oauth::custom_oauth_store_path(NAME)
}

pub fn install_project_and_global(root: &Path, config: &ProviderConfig) -> TestResult {
    let source = super::script::SCRIPT;
    for (directory, script) in [
        (root.join(".fspec/providers"), source.to_owned()),
        (
            root.join("global/providers"),
            source.replace(
                "let response = http::post(config.token_url",
                "throw \"wrong global OAuth script\"; let response = http::post(config.token_url",
            ),
        ),
    ] {
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("selected.rhai"), script)?;
        let mut value = serde_json::to_value(config)?;
        value["script"] = json!("selected.rhai");
        std::fs::write(
            directory.join(format!("{NAME}.json")),
            serde_json::to_vec(&value)?,
        )?;
    }
    Ok(())
}

pub async fn caller_deadline(receiver: tokio::sync::oneshot::Receiver<tokio::time::Instant>) {
    if let Ok(deadline) = receiver.await {
        tokio::time::sleep_until(deadline).await;
    }
}

pub fn failure_provider(
    root: &Path,
    server: &MockServer,
    status: u16,
) -> TestResult<RhaiCustomProvider> {
    let config = config(root, server, super::script::SCRIPT, false)?;
    let mut value = serde_json::to_value(&*config)?;
    if status == 0 {
        value["auth"]["token_url"] = json!("http://127.0.0.1:0/token");
    }
    Ok(RhaiCustomProvider::new(
        Arc::new(serde_json::from_value(value)?),
        Arc::new(ScriptLoader::with_default_engine()),
        "local".into(),
    )?)
}

pub fn tokens(expired: bool) -> Value {
    json!({"access_token":if expired {OLD} else {NEW},"refresh_token":REFRESH,"expires_at":if expired {0} else {9999999999u64}})
}

pub fn seed(expired: bool) -> TestResult {
    let path = token_path();
    std::fs::create_dir_all(path.parent().ok_or("missing token parent")?)?;
    std::fs::write(path, serde_json::to_vec(&tokens(expired))?)?;
    Ok(())
}

pub async fn server() -> TestResult<MockServer> {
    let server = MockServer::start().await;
    assert!(server.uri().starts_with("http://127.0.0.1:"));
    let stored_path = token_path();
    Mock::given(method("GET"))
        .and(path("/stored"))
        .respond_with(move |_: &wiremock::Request| {
            let tokens = std::fs::read(&stored_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .unwrap_or_else(
                    || json!({"access_token":"synthetic-missing","expires_at":9999999999u64}),
                );
            ResponseTemplate::new(200).set_body_json(tokens)
        })
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/inference"))
        .respond_with(|request: &wiremock::Request| {
            let Ok(body) = request.body_json::<Value>() else {
                return ResponseTemplate::new(400);
            };
            if body["stream"] == true {
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: text\n\ndata: usage\n\ndata: stop\n\ndata: [DONE]\n\n")
            } else {
                ResponseTemplate::new(200).set_body_json(json!({"settled":true}))
            }
        })
        .mount(&server)
        .await;
    Ok(server)
}

pub async fn issuer(server: &MockServer, status: u16, delay: Duration) {
    if status == 0 {
        return;
    }
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(status)
                .set_delay(delay)
                .set_body_json(if status == 200 {
                    tokens(false)
                } else {
                    json!({"error":REFRESH,"access_token":OLD})
                }),
        )
        .mount(server)
        .await;
}

pub fn config(
    root: &Path,
    server: &MockServer,
    source: &str,
    custom: bool,
) -> TestResult<Arc<ProviderConfig>> {
    let script_path = root.join(format!("{}.rhai", uuid::Uuid::new_v4()));
    std::fs::write(&script_path, source)?;
    let auth = if custom {
        json!({"type":"custom"})
    } else {
        json!({
            "type":"oauth_device_code","client_id":"synthetic-client","device_code_url":format!("{}/device",server.uri()),
            "token_url":format!("{}/token",server.uri()),"credential_file":"synthetic.json"
        })
    };
    Ok(Arc::new(serde_json::from_value(
        json!({"name":NAME,"display_name":"synthetic","base_url":server.uri(),
        "script":script_path,"auth":auth,"models":{"local":{"id":"synthetic-model","context_window":10000,"max_output_tokens":1000}},
        "default_model":"local","tool_style":"openai"}),
    )?))
}

pub fn provider(
    root: &Path,
    server: &MockServer,
    source: &str,
    custom: bool,
) -> TestResult<RhaiCustomProvider> {
    Ok(RhaiCustomProvider::new(
        config(root, server, source, custom)?,
        Arc::new(ScriptLoader::with_default_engine()),
        "local".into(),
    )?)
}

pub async fn complete(provider: RhaiCustomProvider, door: usize) -> Result<(), String> {
    match door {
        0 => {
            assert_eq!(
                provider
                    .complete(&[codelet_common::Message::user("genuine input")])
                    .await
                    .map_err(|error| error.to_string())?,
                "settled"
            );
        }
        1 => {
            let mut stream = provider
                .complete_with_tools_streaming(
                    &[codelet_common::Message::user("genuine input")],
                    &[],
                )
                .await;
            let mut settled = false;
            while let Some(item) = stream.next().await {
                if matches!(
                    item.map_err(|error| error.to_string())?,
                    codelet_providers::custom::stream::StreamChunk::StopReason(_)
                ) {
                    settled = true;
                }
            }
            assert!(settled);
        }
        door => {
            let model = RhaiCustomProviderModel::new(Arc::new(provider));
            let request = CompletionRequestBuilder::new(model.clone(), "genuine input").build();
            if door == 2 {
                let response = model
                    .completion(request)
                    .await
                    .map_err(|error| error.to_string())?;
                assert_eq!(
                    (response.usage.input_tokens, response.usage.output_tokens),
                    (7, 3)
                );
            } else {
                let mut stream = model
                    .stream(request)
                    .await
                    .map_err(|error| error.to_string())?;
                while let Some(item) = stream.next().await {
                    item.map_err(|error| error.to_string())?;
                }
                let response = stream.response.ok_or("missing final")?;
                assert_eq!((response.input_tokens, response.output_tokens), (7, 3));
            }
        }
    }
    Ok(())
}

pub async fn requests(server: &MockServer, suffix: &str) -> TestResult<Vec<wiremock::Request>> {
    Ok(server
        .received_requests()
        .await
        .ok_or("missing capture")?
        .into_iter()
        .filter(|request| request.url.path() == suffix)
        .collect())
}

pub async fn wait_requests(server: &MockServer, suffix: &str, count: usize) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        while requests(server, suffix).await?.len() < count {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    Ok(())
}

pub async fn wait_persisted(path: &Path) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if std::fs::read(path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .is_some_and(|value| value["access_token"] == NEW)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}

pub async fn assert_inference(server: &MockServer, count: usize, token: &str) -> TestResult {
    let requests = requests(server, "/inference").await?;
    assert_eq!(requests.len(), count);
    for request in requests {
        assert_eq!(request.headers["authorization"], format!("Bearer {token}"));
    }
    Ok(())
}
