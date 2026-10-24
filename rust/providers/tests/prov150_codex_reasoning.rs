//! Feature: spec/features/codex-readable-reasoning.feature
#[path = "fixtures/prov150.rs"]
pub mod fixture;
use fixture::*;
use serde_json::json;

#[tokio::test]
async fn readable_reasoning_streams_incrementally() -> TestResult {
    // @step Given raw reasoning and summary text deltas from Codex
    // @step When the frontend consumes the live stream
    let frontend = run(vec![
        json!({"type":"response.reasoning_text.delta", "item_id":"raw", "content_index":0, "delta":"Consider "}),
        json!({"type":"response.reasoning_text.delta", "item_id":"raw", "content_index":0, "delta":"alternatives"}),
        summary("delta", "summary", 0, "Compare"),
    ]).await?;
    // @step Then each readable delta arrives before the answer completes
    assert_eq!(
        frontend.order,
        [
            "thinking:Consider ",
            "thinking:alternatives",
            "thinking:Compare",
            "answer:Hello"
        ]
    );
    assert!(frontend.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn completed_sections_fill_missing_incremental_delivery() -> TestResult {
    // @step Given summary text done and completed reasoning items without deltas including terminal-only sections
    // @step When the frontend consumes the stream including empty summaries
    let frontend = run(vec![
        summary("done", "first", 0, "First"),
        json!({"type":"response.reasoning_summary_part.done", "item_id":"second", "summary_index":0,
            "part":{"type":"summary_text","text":"Second"}}),
        item("third", &["Third", "Fourth"]),
        item("empty", &[]),
        summary("done", "empty", 0, ""),
    ])
    .await?;
    // @step Then readable sections appear once and empty sections remain invisible
    assert_eq!(
        frontend
            .thinking
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        ["First", "Second", "Third", "Fourth"]
    );
    assert!(frontend.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn repeated_completion_does_not_duplicate_incremental_reasoning() -> TestResult {
    // @step Given a section delivered through deltas and multiple done events
    // @step When the frontend consumes completed items and the terminal response
    let frontend = run(vec![
        summary("delta", "first", 0, "First"),
        summary("done", "first", 0, "First section"),
        summary("done", "first", 0, "First section"),
        item("first", &["First section"]),
    ])
    .await?;
    // @step Then only previously unseen readable text appears
    assert_eq!(
        frontend.order,
        ["thinking:First", "thinking: section", "answer:Hello"]
    );
    assert!(frontend.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn interleaved_reasoning_preserves_section_identity() -> TestResult {
    // @step Given interleaved reasoning items with multiple summary indexes and raw text at the same index
    // @step When the frontend consumes their deltas and completions
    let frontend = run(vec![
        summary("delta", "first", 0, "A"),
        summary("delta", "second", 0, "B"),
        summary("delta", "first", 1, "C"),
        summary("done", "first", 0, "A!"),
        summary("done", "second", 0, "B"),
        summary("done", "first", 1, "C"),
    ])
    .await?;
    // @step Then section identities and arrival order remain distinct
    assert_eq!(
        frontend.order,
        [
            "thinking:A",
            "thinking:B",
            "thinking:C",
            "thinking:!",
            "answer:Hello"
        ]
    );
    assert_eq!(frontend.thinking[0].0, frontend.thinking[3].0);
    assert_ne!(frontend.thinking[0].0, frontend.thinking[1].0);
    assert_ne!(frontend.thinking[0].0, frontend.thinking[2].0);
    Ok(())
}

#[tokio::test]
async fn encrypted_reasoning_never_reaches_presentation_or_diagnostics() -> TestResult {
    // @step Given encrypted reasoning alongside readable summaries
    // @step When valid and malformed reasoning events reach the frontend
    let frontend = run(vec![item("safe", &["Readable"])]).await?;
    // @step Then readable text is visible and encrypted payloads never appear in output or errors
    assert_eq!(frontend.order, ["thinking:Readable", "answer:Hello"]);
    assert!(!format!("{frontend:?}").contains("opaque-secret"));
    for malformed in [
        json!({"type":"response.reasoning_text.delta","item_id":"bad", "content_index":0, "delta":{"encrypted_content":"opaque-secret"}}),
        json!({"type":"response.reasoning_summary_text.done","item_id":"bad", "summary_index":0, "text":42,"encrypted_content":"opaque-secret"}),
        json!({"type":"response.reasoning_summary_text.delta","item_id":"bad", "summary_index":"opaque-secret", "delta":"Readable"}),
    ] {
        let frontend = run(vec![malformed]).await?;
        assert_eq!(frontend.errors.len(), 1);
        assert!(!format!("{frontend:?}").contains("opaque-secret"));
    }
    Ok(())
}

#[tokio::test]
async fn ordinary_responses_thinking_remains_unchanged() -> TestResult {
    // @step Given identical done-only reasoning streams for Codex and ordinary Responses
    let body = event(item("safe", &["Readable"]))
        + text().as_str()
        + terminal("completed", Some(true), None).as_str();
    let server = Server::start(vec![body.clone(), body], false).await?;
    let (codex, ordinary) = server.models()?;
    // @step When both frontends consume the streams
    let codex = consume(codex).await?;
    let ordinary = consume(ordinary).await?;
    // @step Then only Codex presents completed reasoning as thinking deltas
    assert_eq!(codex.order, ["thinking:Readable", "answer:Hello"]);
    assert_eq!(ordinary.order, ["answer:Hello"]);
    assert!(ordinary.errors.is_empty());
    Ok(())
}
