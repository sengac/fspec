//! Feature: spec/features/codex-optional-reasoning-identity.feature
#[path = "fixtures/prov150.rs"]
pub mod fixture;
use fixture::*;
use serde_json::json;

#[tokio::test]
async fn optional_reasoning_identities_retain_compatibility() -> TestResult {
    // @step Given reasoning events without optional item IDs at distinct output positions
    let first = json!({"type":"reasoning", "id":"output:2", "summary":[
        {"type":"summary_text", "text":"First!"}]});
    let second = json!({"type":"reasoning", "id":"second", "summary":[
        {"type":"summary_text", "text":"Second!"}]});
    let body = [
        event(
            json!({"type":"response.reasoning_summary_text.delta", "output_index":0,
            "summary_index":0, "delta":"First"}),
        ),
        event(
            json!({"type":"response.reasoning_summary_text.delta", "output_index":1,
            "summary_index":0, "delta":"Second"}),
        ),
        event(json!({"type":"response.output_item.done", "output_index":0, "item":first})),
        event(
            json!({"type":"response.reasoning_summary_text.done", "item_id":"output:2",
            "summary_index":0, "text":"First!"}),
        ),
        event(json!({"type":"response.output_item.done", "output_index":1, "item":second})),
        event(
            json!({"type":"response.completed", "response":{"id":"response", "status":"completed",
            "output":[first, second]}}),
        ),
    ]
    .concat();
    let server = Server::start(vec![body], false).await?;
    // @step When completed items identify those positions and repeat their readable text
    let frontend = consume(server.models()?.0).await?;
    // @step Then each section keeps a stable identity and only unseen text appears
    assert!(frontend.errors.is_empty(), "{:?}", frontend.errors);
    assert_eq!(
        frontend.order,
        [
            "thinking:First",
            "thinking:Second",
            "thinking:!",
            "thinking:!"
        ]
    );
    assert_eq!(frontend.thinking[0].0, frontend.thinking[2].0);
    assert_eq!(frontend.thinking[1].0, frontend.thinking[3].0);
    assert_ne!(frontend.thinking[0].0, frontend.thinking[1].0);
    assert_eq!(
        server.requests.lock().map_err(|_| "requests lock")?.len(),
        1
    );
    Ok(())
}
