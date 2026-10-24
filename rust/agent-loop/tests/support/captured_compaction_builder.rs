use super::{append_original, load_session, support, RAW_D2};
use codelet_agent_loop::generate_compaction_handler::execute_compaction_subagent;
use codelet_tools::SessionSearchTool;
use rig::tool::Tool;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Drop for Environment {
    fn drop(&mut self) {
        for (key, previous) in &self.0 {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn builder_tool_reads_the_captured_prefix_not_its_live_suffix() -> support::TestResult {
    let (directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_original(&mut manifest, "captured evidence sentinel")?;
    let server = MockServer::start().await;
    let _environment = Environment(
        ["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect(),
    );
    std::env::set_var("OPENAI_API_KEY", "test-captured-builder");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    std::env::set_var("OPENAI_MODEL", "o3");
    let source_id = manifest.id;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let arguments = serde_json::to_string(&json!({
        "action_type":"originals", "session_id":source_id,
        "limit":100, "max_bytes":61440
    }))?;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let first = observed.fetch_add(1, Ordering::SeqCst) == 0;
            let message = if first {
                let appended = load_session(source_id)
                    .and_then(|mut live| append_original(&mut live, "live append sentinel"));
                if appended.is_err() {
                    return ResponseTemplate::new(500);
                }
                json!({"role":"assistant", "content":null, "tool_calls":[{
                    "id":"captured-originals", "type":"function",
                    "function":{"name":SessionSearchTool::NAME, "arguments":arguments}
                }]})
            } else {
                json!({"role":"assistant", "content":RAW_D2})
            };
            ResponseTemplate::new(200).set_body_json(json!({
                "id":"chatcmpl-captured", "object":"chat.completion", "created":1,
                "usage":{"prompt_tokens":100, "completion_tokens":20, "total_tokens":120},
                "model":"o3", "choices":[{"index":0, "message":message,
                    "finish_reason":if first {"tool_calls"} else {"stop"}}]
            }))
        })
        .mount(&server)
        .await;
    let output = execute_compaction_subagent(
        directory.path(),
        source_id,
        None,
        &Arc::new(AtomicBool::new(true)),
        "openai",
        Some("o3"),
        Some(100_000),
        Some(4096),
    )
    .await?;
    assert_eq!(output, RAW_D2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(load_session(source_id)?.messages.len(), 2);
    let requests = server
        .received_requests()
        .await
        .ok_or("captured requests")?;
    let second: serde_json::Value = requests
        .get(1)
        .ok_or("second builder request")?
        .body_json()?;
    let encoded = serde_json::to_string(&second)?;
    assert!(encoded.contains("captured evidence sentinel"), "{encoded}");
    assert!(!encoded.contains("live append sentinel"), "{encoded}");
    Ok(())
}
