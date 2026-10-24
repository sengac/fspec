use codelet_providers::codex::history::CodexHistory;
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

fn terminal(status: &str, output: Vec<Value>) -> String {
    fixture::event(json!({"type":format!("response.{status}"),"response":{
        "id":"resp_compact","status":status,"end_turn":true,"output":output,
        "error": if status == "failed" { json!({"code":"failed","message":"rejected"}) } else { Value::Null },
        "incomplete_details": if status == "incomplete" { json!({"reason":"interrupted"}) } else { Value::Null }
    }}))
}

#[tokio::test]
async fn unsolicited_native_compaction_never_replaces_the_checkpoint() -> fixture::TestResult {
    for status in ["completed", "failed", "incomplete"] {
        let opaque = json!({"type":"compaction","id":"cmp_transport","encrypted_content":"opaque-transport+/=秘密"});
        let server = fixture::Server::start(
            vec![
                terminal("completed", vec![]),
                terminal(status, vec![opaque]),
            ],
            false,
        )
        .await?;
        let history = CodexHistory::default();
        let model = server.models()?.0.with_history(history.clone());
        let initial = fixture::collect(model.clone()).await?;
        assert!(initial.errors.is_empty());
        let prior = history.snapshot();
        let rejected = fixture::collect(model).await?;
        assert_eq!(rejected.errors.len(), 1);
        assert!(rejected.errors[0].contains("native memory unsupported"));
        assert!(rejected.finals.is_empty());
        assert_eq!(rejected.tools, 0);
        assert_eq!(history.snapshot(), prior);
        let requests = server.requests.lock().map_err(|_| "requests lock")?;
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            assert!(request.get("context_management").is_none());
            assert!(!request.to_string().contains("compaction_trigger"));
        }
    }
    Ok(())
}
