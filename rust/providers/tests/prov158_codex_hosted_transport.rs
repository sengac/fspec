use codelet_providers::codex::NativeCapabilities;
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::Message,
};
use serde_json::json;

#[path = "fixtures/prov148.rs"]
pub mod fixture;

#[tokio::test]
async fn negotiated_hosted_search_presents_attributable_status_and_sources() -> fixture::TestResult
{
    // @step Given a Codex session explicitly negotiating hosted search with replay support
    let search = json!({"type":"web_search_call","id":"ws_transport","status":"completed",
        "action":{"type":"search","query":"Rust"},"encrypted_content":"private-search"});
    let message = json!({"type":"message","id":"msg_transport","role":"assistant",
        "content":[{"type":"output_text","text":"Rust source","annotations":[{
            "type":"url_citation","url":"https://rust-lang.org","title":"Rust",
            "start_index":0,"end_index":11}]}]});
    let body = [
        fixture::event(
            json!({"type":"response.web_search_call.searching","item_id":"ws_transport"}),
        ),
        fixture::event(json!({"type":"response.output_item.done","output_index":0,"item":search})),
        fixture::event(
            json!({"type":"response.completed","response":{"id":"resp_hosted",
            "status":"completed","end_turn":true,"output":[search.clone(),message.clone()],
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}}}),
        ),
    ]
    .concat();
    let server = fixture::Server::start(
        vec![body, fixture::terminal("completed", Some(true), None)],
        false,
    )
    .await?;
    let history = codelet_providers::codex::history::CodexHistory::default();
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        });
    // @step When hosted search progress completion and source citations arrive
    let observed = fixture::collect(model.clone()).await?;
    assert!(observed.errors.is_empty(), "{:?}", observed.errors);
    // @step And no hosted call is dispatched as a local tool
    assert_eq!(observed.tools, 0);
    assert_eq!(observed.finals.len(), 1);
    // @step Then visible output attributes status and sources to the hosted item
    assert!(observed.reasoning.contains("ws_transport: searching"));
    assert_eq!(
        observed
            .reasoning
            .matches("ws_transport: completed")
            .count(),
        1
    );
    assert!(observed.reasoning.contains("msg_transport"));
    assert!(observed.reasoning.contains("https://rust-lang.org"));
    assert!(observed.text.contains("Rust source"));
    assert!(!observed.text.contains("private-search"));
    let saved = history.snapshot()?.ok_or("hosted checkpoint missing")?;
    assert!(!format!("{saved:?}").contains("private-search"));
    // @step And its typed output survives continuation
    let request = CompletionRequestBuilder::new(model.clone(), "Continue")
        .preamble("Test instructions".to_string())
        .messages(vec![
            Message::user("Hello"),
            Message::assistant(&observed.text),
        ])
        .build();
    let mut replay = model.stream(request).await?;
    while let Some(event) = replay.next().await {
        event?;
    }
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    let input = requests[1]["input"].as_array().ok_or("input")?;
    assert!(input.contains(&search));
    assert!(input.contains(&message));
    Ok(())
}
