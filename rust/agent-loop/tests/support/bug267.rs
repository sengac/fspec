use super::{
    environment::{Environment, TestResult},
    wire,
};
use codelet_agent_loop::agent_loop::agent_loop;
use codelet_cli::session::Session;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use serde_json::{json, Value};
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{broadcast, mpsc};
use wiremock::{matchers::method, Mock, MockServer};

struct RunningLoop(tokio::task::JoinHandle<()>);

impl Drop for RunningLoop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub async fn run_case(
    name: &str,
    register: bool,
    disabled: bool,
    facade: Option<&str>,
) -> TestResult {
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = MockServer::start().await;
    std::env::set_var("OPENAI_BASE_URL", format!("{}/v1", server.uri()));
    if disabled {
        std::env::set_var("FSPEC_DISABLE_SCRIPT_SHADOWING", "1");
    }
    if register {
        let providers = root.join(".fspec/providers");
        std::fs::create_dir_all(&providers)?;
        std::fs::write(providers.join("script.rhai"), wire::SCRIPT)?;
        std::fs::write(
            providers.join(format!("{name}.json")),
            serde_json::to_vec(&json!({
                "name":name,"display_name":name,"base_url":server.uri(),"script":"script.rhai",
                "models":{"smart":{"id":"wire-model","context_window":200000,"max_output_tokens":4096}}
            }))?,
        )?;
    }
    let expected_custom = register && !disabled;
    let selected = ProviderType::from_str(name)?;
    if expected_custom {
        assert_eq!(selected, ProviderType::Custom(name.into()));
    } else {
        assert_eq!(selected, ProviderType::OpenAI);
    }
    let mut manager = ProviderManager::for_testing(selected, Some(200_000), Some(4096));
    manager.set_model_direct(
        name,
        if expected_custom {
            "smart"
        } else {
            "wire-model"
        },
        Some(200_000),
        Some(4096),
        facade.map(str::to_owned),
    )?;
    assert_eq!(
        manager.current_provider_type(),
        &ProviderType::from_str(name)?
    );
    let inner = Session::from_provider_manager(manager);
    let manifest = codelet_core::persistence::create_session_with_provider(
        "selected implementation",
        root,
        name,
    )?;
    let (input_tx, input_rx) = mpsc::channel(4);
    let (chunks_tx, mut chunks_rx) = broadcast::channel(256);
    let background = Arc::new(BackgroundSession::new(
        manifest.id,
        "selected implementation".into(),
        root.display().to_string(),
        None,
        None,
        inner,
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        broadcast::channel(32).0,
        broadcast::channel(4).0,
    ));
    background.set_continue_state(true, 1);
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(background.id, root.to_owned())?;
    let armed = Arc::new(AtomicBool::new(false));
    let observed = armed.clone();
    let session_id = background.id;
    Mock::given(method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            observed.store(
                codelet_tools::is_continue_armed(session_id),
                Ordering::SeqCst,
            );
            match serde_json::from_slice::<Value>(&request.body) {
                Ok(body) => wire::response(&body),
                Err(_) => wiremock::ResponseTemplate::new(400),
            }
        })
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let (_injection, injection_rx) = mpsc::channel(4);
    let running = RunningLoop(tokio::spawn(agent_loop(
        background.clone(),
        input_rx,
        injection_rx,
    )));
    background.send_input("finish the selected task".into(), None)?;
    let chunks = tokio::time::timeout(Duration::from_secs(10), async {
        let mut chunks = Vec::new();
        loop {
            let (_, chunk) = chunks_rx.recv().await?;
            let done = matches!(chunk, StreamChunk::Done);
            chunks.push(chunk);
            if done {
                return Ok::<_, broadcast::error::RecvError>(chunks);
            }
        }
    })
    .await??;
    drop(running);
    let requests = server
        .received_requests()
        .await
        .ok_or("missing request capture")?;
    if expected_custom && facade.is_some() {
        assert!(
            requests.is_empty(),
            "incompatible explicit facade sent HTTP"
        );
        assert!(chunks.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error } if error.contains("Current provider is not OpenAI"))), "{chunks:?}");
        return Ok(());
    }
    assert_eq!(requests.len(), 1, "{name}: {chunks:?}");
    assert!(armed.load(Ordering::SeqCst));
    assert!(
        !chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::Error { .. })),
        "{chunks:?}"
    );
    let body: Value = requests[0].body_json()?;
    if expected_custom {
        assert_eq!(requests[0].url.path(), "/script");
        assert_eq!(body["script_marker"], "selected-script");
        assert_eq!(requests[0].headers["x-script-model"], "wire-model");
    } else {
        assert_eq!(requests[0].url.path(), "/v1/chat/completions");
        assert!(body.get("script_marker").is_none());
        assert_eq!(body["model"], "wire-model");
    }
    let done = body["tools"]
        .as_array()
        .ok_or("missing tools")?
        .iter()
        .map(|tool| tool.get("function").unwrap_or(tool))
        .find(|tool| tool["name"] == "Done")
        .ok_or("missing native Done")?;
    let schema = done
        .get("parameters")
        .or_else(|| done.get("input_schema"))
        .ok_or("missing schema")?;
    assert!(schema["required"]
        .as_array()
        .ok_or("missing required")?
        .iter()
        .any(|name| name == "work_unit_id"));
    let serialized = serde_json::to_string(&chunks)?;
    assert!(serialized.contains("Completion recorded"), "{serialized}");
    assert!(!serialized.contains("invalid work_unit_id"));
    assert_eq!(background.inner.lock().await.continue_nudges_used, 0);
    let persisted = codelet_core::persistence::load_session(session_id)?;
    assert_eq!(persisted.token_usage.cumulative_billed_input, 12);
    assert_eq!(persisted.token_usage.cumulative_billed_output, 3);
    Ok(())
}
