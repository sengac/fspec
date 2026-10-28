use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::codex::{history::CodexHistory, session::CodexSession, CodexResponsesModel};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{CodexProvider, ProviderManager, ProviderType};
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{AssistantContent, Message};
use serde_json::{json, Value};
use std::collections::HashMap;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const MODEL: &str = "gpt-6.1-sol";
pub const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">original task</dag-node>";

pub fn runtime() -> Result<Session, Box<dyn std::error::Error>> {
    let mut manager = ProviderManager::for_testing(ProviderType::Codex, Some(200_000), Some(4096));
    manager.set_model_direct("codex", MODEL, Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    let provider = CodexProvider::from_api_key("synthetic-offline-key", MODEL)?;
    let model =
        CodexResponsesModel::new(provider.client().clone(), MODEL).with_history(history(&session)?);
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    session.install_selected_preparer(SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    ));
    Ok(session)
}

pub async fn continue_cold(session: &Session, messages: &[Message]) -> TestResult {
    use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
    use futures::StreamExt;
    use rig::streaming::StreamedAssistantContent;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let response = json!({"type":"response.completed","response":{
        "id":"synthetic-continuation","status":"completed","output":[],"end_turn":true,
        "usage":{"input_tokens":12,"output_tokens":1,"total_tokens":13}}});
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("event: response.completed\ndata: {response}\n\n")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("synthetic-offline-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, MODEL)
        .with_history(history(session)?)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        });
    let mut request = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    request.chat_history = rig::OneOrMany::many(messages.to_vec())?;
    let mut stream = model.stream(request).await?;
    let mut completed = false;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::ToolCall(_) => {
                return Err("completed effect was requested again".into())
            }
            StreamedAssistantContent::Final(_) => completed = true,
            _ => {}
        }
    }
    assert!(completed);
    let requests = server.received_requests().await.ok_or("captured request")?;
    let physical: Value = serde_json::from_slice(&requests[0].body)?;
    assert_eq!(physical["model"], MODEL);
    assert_eq!(
        physical["input"]
            .as_array()
            .ok_or("input")?
            .iter()
            .filter(|item| item["type"] == "function_call_output")
            .count(),
        1
    );
    assert!(physical.to_string().contains("genuine effect=1"));
    Ok(())
}

pub fn reconcile_failure(
    fixture: &mut Fixture,
    error: codelet_core::persistence::compaction_commit::CompactionCommitError,
) -> TestResult {
    let identity = error
        .promoted_identity()
        .ok_or_else(|| format!("not promoted: {error}"))?
        .clone();
    assert!(fixture.session.ensure_compaction_inference_ready().is_err());
    let mut wrong = identity.clone();
    wrong.operation_id = uuid::Uuid::new_v4();
    assert!(fixture
        .session
        .reconcile_durable_compaction(&wrong)
        .is_err());
    assert!(fixture.session.ensure_compaction_inference_ready().is_err());
    fixture.session.reconcile_durable_compaction(&identity)?;
    fixture.session.ensure_compaction_inference_ready()?;
    let boundary = load_session(fixture.manifest.id)?
        .compaction_commit
        .ok_or("boundary")?;
    assert_eq!(boundary.identity, identity);
    assert!(!boundary.publication_pending);
    Ok(())
}

pub fn history(session: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("selected Codex extension")?
        .history
        .clone())
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub session: Session,
    pub manifest: SessionManifest,
}

pub fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut session = runtime()?;
    session.reset_provider_state()?;
    let mut manifest = create_session_with_provider(
        "BUG-278 synthetic lineage",
        directory.path(),
        &format!("codex/{MODEL}"),
    )?;
    append_message(&mut manifest, "user", "original task")?;
    session.bind_provider_persistence(manifest.id)?;
    session.messages.push(Message::user("original task"));
    history(&session)?.commit(&wire(&session.messages)?, &[])?;
    manifest = load_session(manifest.id)?;
    Ok(Fixture {
        directory,
        session,
        manifest,
    })
}

pub fn append_pair(manifest: &mut SessionManifest, call: &str, outcome: bool) -> TestResult {
    use std::io::Write;
    let mut effect = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest.project.join(format!("effect-{call}")))?;
    effect.write_all(b"genuine effect=1")?;
    append_message_with_metadata(
        manifest,
        "assistant",
        "",
        HashMap::from([(
            "message".into(),
            json!({"role":"assistant","content":[{"type":"tool_use","id":format!("item-{call}"),
            "call_id":call,"name":"shell_command","input":{"command":"effect"}}]}),
        )]),
    )?;
    if outcome {
        append_message_with_metadata(
            manifest,
            "user",
            "",
            HashMap::from([(
                "message".into(),
                json!({"role":"user","content":[{"type":"tool_result","tool_use_id":format!("item-{call}"),
                "call_id":call,"content":std::fs::read_to_string(manifest.project.join(format!("effect-{call}")))?}]}),
            )]),
        )?;
    }
    Ok(())
}

pub fn pair(call: &str) -> Vec<Message> {
    vec![
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                format!("item-{call}"),
                call.into(),
                "shell_command",
                json!({"command":"effect"}),
            )),
        },
        Message::tool_result_with_call_id(
            format!("item-{call}"),
            Some(call.into()),
            "genuine effect=1",
        ),
    ]
}

pub fn wire(messages: &[Message]) -> Result<Value, Box<dyn std::error::Error>> {
    let provider = CodexProvider::from_api_key("synthetic-offline-key", MODEL)?;
    let model = CodexResponsesModel::new(provider.client().clone(), MODEL);
    let mut request = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    request.chat_history = rig::OneOrMany::many(messages.to_vec())?;
    Ok(serde_json::from_slice(&model.prepare_request(request)?)?)
}

pub fn checkpoint_pair(session: &Session, call: &str) -> TestResult {
    let mut messages = session.messages.clone();
    messages.extend(pair(call));
    history(session)?.commit(&wire(&messages)?, &[])?;
    Ok(())
}

pub fn original_bytes(manifest: &SessionManifest) -> Result<Value, Box<dyn std::error::Error>> {
    let originals = manifest
        .messages
        .iter()
        .map(|reference| {
            get_message(reference.message_id)?.ok_or_else(|| "missing original".to_owned())
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(serde_json::to_value(originals)?)
}

pub fn assert_refused(
    fixture: &mut Fixture,
    source: codelet_core::persistence::compaction_boundary::CompactionSource,
) -> Result<String, Box<dyn std::error::Error>> {
    let before = serde_json::to_value(load_session(fixture.manifest.id)?)?;
    let mut published = false;
    let result =
        fixture
            .session
            .commit_selected_compaction(source, uuid::Uuid::new_v4(), DAG, |_| {
                published = true;
                Ok(())
            });
    let failure = result.err().ok_or("unproven candidate promoted")?;
    assert!(!published);
    assert_eq!(
        serde_json::to_value(load_session(fixture.manifest.id)?)?,
        before
    );
    assert!(fixture.session.ensure_compaction_inference_ready().is_err());
    Ok(failure.to_string())
}
