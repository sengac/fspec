use codelet_cli::session::Session;
pub use codelet_core::persistence::*;
pub use codelet_providers::codex::history::CodexHistory;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
pub use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
pub use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn setup() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    Ok(directory)
}

pub fn session() -> Session {
    Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Codex,
        Some(272_000),
        Some(4096),
    ))
}

pub fn history(session: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension")?
        .history
        .clone())
}

pub fn request(
    session_id: Uuid,
    next_turn: Option<usize>,
) -> codelet_tools::session_search::originals::OriginalsRequest {
    codelet_tools::session_search::originals::OriginalsRequest {
        session_id: session_id.to_string(),
        start_turn: Some(0),
        end_turn: None,
        next_turn,
        limit: Some(1),
        max_bytes: Some(61440),
    }
}

pub fn state_path(root: &Path, session_id: Uuid) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_slice(&std::fs::read(
        root.join(format!("sessions/{session_id}.json")),
    )?)?;
    let state_id = manifest["provider_state"]["state_id"]
        .as_str()
        .ok_or("missing committed provider-state reference")?;
    Ok(root
        .join("provider-state")
        .join(session_id.to_string())
        .join(format!("{state_id}.json")))
}

pub fn dialogue(root: &Path) -> Result<(SessionManifest, Vec<Value>), Box<dyn std::error::Error>> {
    let mut manifest = create_session_with_provider("checkpoint originals", root, "codex")?;
    let mut originals = Vec::new();
    for (role, content) in [
        ("user", json!([{"type":"text","text":"seed"}])),
        (
            "assistant",
            json!([{"type":"tool_use","id":"item-exact","call_id":"call-exact","name":"inspect","input":{}}]),
        ),
        (
            "user",
            json!([{"type":"tool_result","tool_use_id":"item-exact","call_id":"call-exact","content":"authentic output"}]),
        ),
    ] {
        let envelope = json!({"uuid":Uuid::new_v4(),"provider":"codex","message":{"role":role,"content":content}});
        append_message_with_metadata(
            &mut manifest,
            role,
            "",
            serde_json::from_value(envelope.clone())?,
        )?;
        originals.push(envelope);
    }
    Ok((manifest, originals))
}

pub fn native(secret: &str) -> Value {
    json!([
        {"type":"reasoning","id":"reason-exact","encrypted_content":secret,"summary":[]},
        {"type":"function_call","id":"item-exact","call_id":"call-exact","name":"inspect","arguments":"{}"},
        {"type":"function_call_output","call_id":"call-exact","output":"authentic output"}
    ])
}

pub fn publish(history: &CodexHistory, secret: &str) -> TestResult {
    history.commit(&json!({"model":"shared-model","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"seed"}]}]}), native(secret).as_array().ok_or("native array")?)?;
    Ok(())
}

pub fn legacy(root: &Path) -> Result<(SessionManifest, Value), Box<dyn std::error::Error>> {
    let (mut manifest, _) = dialogue(root)?;
    let source = CodexHistory::default();
    publish(&source, &"secret-native".repeat(100_000))?;
    let checkpoint = serde_json::to_value(source.snapshot()?)?;
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "",
        HashMap::from([
            ("provider".into(), json!("codex")),
            ("message".into(), json!({"role":"assistant","content":[]})),
            ("codexContinuation".into(), checkpoint.clone()),
            ("_codexReset".into(), Value::Null),
        ]),
    )?;
    Ok((manifest, checkpoint))
}
