use codelet_providers::{
    codex::{history::CodexHistory, session::CodexSession},
    session::ProviderSession,
};
use serde_json::{json, Value};

pub fn request(text: &str) -> Value {
    json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":text}]}]})
}

pub fn history(session: &ProviderSession) -> Result<&CodexHistory, Box<dyn std::error::Error>> {
    Ok(&session
        .extension::<CodexSession>("codex")
        .ok_or("Codex session missing")?
        .history)
}

pub fn boundary() -> Result<Value, Box<dyn std::error::Error>> {
    let session = ProviderSession::for_provider("codex");
    let metadata = session.prepare_generation(1, "shared-model")?;
    Ok(json!({
        "_synthetic":true,"_compactionSummary":true,
        "_committedCompaction":{"version":1,"generation":1,"revision":2,"operationId":"commit-1"},
        "codexContinuation":metadata["codexContinuation"],
        "message":{"role":"user","content":"committed DAG"}
    }))
}
