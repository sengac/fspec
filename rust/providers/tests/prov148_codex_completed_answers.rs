//! Feature: spec/features/codex-terminal-lifecycle.feature
#[path = "fixtures/prov148.rs"]
pub mod fixture;
use fixture::*;
use serde_json::json;

#[tokio::test]
async fn completed_answers_fill_missing_stream_delivery() -> TestResult {
    // @step Given completed answer and refusal sections with absent or partial deltas
    for prefix in ["", "秘密", "秘密!"] {
        let message = json!({"type":"message", "id":"message", "role":"assistant",
            "content":[{"type":"output_text", "text":"秘密!"},
                {"type":"refusal", "refusal":"No"},
                {"type":"output_text", "text":"Last"}]});
        let delta = if prefix.is_empty() {
            String::new()
        } else {
            event(
                json!({"type":"response.output_text.delta", "output_index":0,
                "content_index":0, "delta":prefix}),
            )
        };
        for item_done in [false, true] {
            let done = if item_done {
                event(json!({"type":"response.output_item.done", "output_index":0, "item":message}))
            } else {
                String::new()
            };
            let body = [
                delta.clone(),
                done,
                event(
                    json!({"type":"response.completed", "response":{"id":"response",
                    "status":"completed", "end_turn":true, "output":[message],
                    "usage":baseline::response()["usage"]}}),
                ),
            ]
            .concat();
            // @step When item completion and successful terminal output repeat those sections
            let result = run(body).await?;
            // @step Then each unseen answer suffix appears once without another request
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            assert_eq!(
                result.text, "秘密!NoLast",
                "prefix={prefix}, item_done={item_done}"
            );
            assert_eq!(result.finals.len(), 1);
        }
    }
    Ok(())
}
