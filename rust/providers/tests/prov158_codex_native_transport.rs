use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

fn terminal(output: Vec<Value>) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_native","status":"completed","end_turn":true,"output":output,
        "usage":fixture::baseline::response()["usage"]
    }}))
}

#[tokio::test]
async fn unsupported_native_output_never_completes_or_dispatches() -> fixture::TestResult {
    // @step Given a response containing known deferred or unknown executable output
    // @step When the Codex adapter consumes the response
    // @step Then an actionable compatibility diagnostic names the unsupported family
    // @step And no local execution or successful completion is reported
    for family in [
        "image_generation_call",
        "future_executor",
        "web_search_call",
        "compaction",
    ] {
        let observed = fixture::run(format!(
            "{}{}",
            fixture::text(),
            terminal(vec![
                json!({"type":family,"id":"item_1","encrypted_content":"secret-native"})
            ])
        ))
        .await?;
        assert_eq!(observed.tools, 0);
        assert!(observed.finals.is_empty());
        assert_eq!(observed.errors.len(), 1);
        assert!(observed.errors[0].contains(family), "{:?}", observed.errors);
        assert!(
            observed.errors[0].contains("disable"),
            "{:?}",
            observed.errors
        );
        assert!(!observed.errors[0].contains("secret-native"));
    }
    Ok(())
}

#[tokio::test]
async fn optional_metadata_preserves_usable_text_without_native_capabilities() -> fixture::TestResult
{
    // @step Given a Codex text response with unknown optional metadata
    // @step When the adapter consumes the response without native capabilities
    // @step Then the visible text and normal completion remain usable
    let observed = fixture::run(format!(
        "{}{}",
        fixture::text(),
        terminal(vec![
            json!({"type":"message","id":"msg_1","role":"assistant",
            "unknown_metadata":{"future":"ok"},
            "content":[{"type":"output_text","text":"Hello","annotations":[]}]})
        ])
    ))
    .await?;
    assert_eq!(observed.text, "Hello");
    assert!(observed.errors.is_empty());
    assert_eq!(observed.tools, 0);
    assert_eq!(observed.finals.len(), 1);
    Ok(())
}
