use super::{Fixture, Result};
use codelet_rpc_types::SessionId;
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn verify(fixture: &Fixture) -> Result {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let args = json!({"work_unit_id":"AUTH-002","command":"update-work-unit-status",
        "args":json!({"workUnitId":"AUTH-002","status":"validating"}).to_string(),
        "project_root":fixture.project.path().to_string_lossy()})
    .to_string();
    Mock::given(method("POST")).respond_with(move |_: &wiremock::Request| {
        let delta = if observed.fetch_add(1, Ordering::SeqCst) == 0 {
            json!({"role":"assistant", "tool_calls":[{"index":0,"id":"focus-call","type":"function", "function":{"name":"fspec","arguments":args}}]})
        } else {
            json!({"role":"assistant","content":"finished"})
        };
        let finish = if delta.get("tool_calls").is_some() { "tool_calls" } else { "stop" };
        let chunk = json!({"id":"focus-turn","object":"chat.completion.chunk","created":1,"model":"o3",
            "choices":[{"index":0,"delta":delta,"finish_reason":finish}],
            "usage":{"prompt_tokens":50,"completion_tokens":5,"total_tokens":55}});
        ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
    }).mount(&server).await;
    let previous: Vec<_> = ["OPENAI_API_KEY", "OPENAI_BASE_URL"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    std::env::set_var("OPENAI_API_KEY", "offline");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    fixture
        .manager
        .set_hooks(Arc::new(codelet_agent_loop::FspecAgentHooks::new()));
    let id = super::create(&fixture.manager, fixture.project.path()).await?;
    let session = fixture.manager.get_session(&id.value)?;
    session
        .inner
        .lock()
        .await
        .provider_manager_mut()
        .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    session.send_input("Update AUTH-002 status".into(), None)?;
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if fixture
                .focus(&SessionId::from(session.id.to_string()))?
                .as_deref()
                == Some("AUTH-002")
            {
                return Ok::<(), Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    session.interrupt();
    for (key, value) in previous {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    result??;
    assert!(calls.load(Ordering::SeqCst) > 0);
    Ok(())
}
