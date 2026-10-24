//! Feature: spec/features/codex-completed-reasoning.feature
#[path = "fixtures/prov150.rs"]
pub mod fixture;
use fixture::*;
use serde_json::json;

#[tokio::test]
async fn terminal_and_raw_completed_reasoning_fills_missing_delivery() -> TestResult {
    // @step Given raw reasoning completion and terminal-only summary sections
    // @step When the Codex frontend consumes the response
    let frontend = run(vec![
        json!({"type":"response.reasoning_text.done", "item_id":"raw", "content_index":0, "text":"Raw"}),
        json!({"type":"response.output_item.done", "item":{"type":"reasoning","id":"raw-item",
            "summary":[],"content":[{"type":"reasoning_text","text":"Content"}]}}),
    ]).await?;
    // @step Then each completed readable section appears without requiring a delta
    assert_eq!(
        frontend.order,
        ["thinking:Raw", "thinking:Content", "answer:Hello"]
    );
    assert!(frontend.errors.is_empty());
    let mut response: serde_json::Value = serde_json::from_str(
        terminal("completed", Some(true), None)
            .trim()
            .trim_start_matches("data: "),
    )?;
    response["response"]["output"] = json!([item("terminal", &["Terminal"])["item"]]);
    let server = Server::start(vec![event(response)], false).await?;
    let frontend = consume(server.models()?.0).await?;
    assert_eq!(frontend.order, ["thinking:Terminal"]);
    assert!(frontend.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn raw_and_summary_reasoning_retain_distinct_identities() -> TestResult {
    // @step Given raw reasoning and summary text interleaved at the same item index
    // @step When both sections complete with additional Unicode text
    let frontend = run(vec![
        summary("delta", "first", 0, "Summary"),
        json!({"type":"response.reasoning_text.delta", "item_id":"first", "content_index":0, "delta":"秘密"}),
        json!({"type":"response.reasoning_text.done", "item_id":"first", "content_index":0, "text":"秘密!"}),
        summary("done", "first", 0, "Summary!"),
    ]).await?;
    // @step Then only unseen text appears under the original distinct section identities
    assert_eq!(
        frontend.order,
        [
            "thinking:Summary",
            "thinking:秘密",
            "thinking:!",
            "thinking:!",
            "answer:Hello"
        ]
    );
    assert_ne!(frontend.thinking[0].0, frontend.thinking[1].0);
    assert_eq!(frontend.thinking[1].0, frontend.thinking[2].0);
    assert_eq!(frontend.thinking[0].0, frontend.thinking[3].0);
    assert!(frontend.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn completed_presentation_can_be_disabled_independently() -> TestResult {
    // @step Given completed-section presentation disabled for Codex
    let body = [
        event(summary("delta", "first", 0, "Plan")),
        event(summary("done", "first", 0, "Plan more")),
        event(item("second", &["Completed only"])),
        text(),
        terminal("completed", Some(true), None),
    ]
    .concat();
    let server = Server::start(vec![body], false).await?;
    // @step When reasoning deltas completed sections and answer deltas arrive
    let frontend = consume(server.models()?.0.with_completed_reasoning(false)).await?;
    // @step Then incremental reasoning and answers still stream without completed-only text
    assert_eq!(frontend.order, ["thinking:Plan", "answer:Hello"]);
    assert!(frontend.errors.is_empty());
    Ok(())
}
