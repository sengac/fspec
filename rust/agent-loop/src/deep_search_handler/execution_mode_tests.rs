//! Feature: spec/features/deep-search.feature
//!
//! BUG-514: Scenario "Sub-agent runs in its provider-specific execution mode",
//! checked against the code that chooses the mode (provider_uses_streaming_execution)
//! and runs it (collect_final_response_from_stream), with a synthetic
//! multi-turn stream standing in for the provider.
use super::*;
use rig::agent::MultiTurnStreamItem;
use rig::message::{ToolCall, ToolFunction, ToolResult, ToolResultContent};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::OneOrMany;

#[derive(Clone)]
struct SyntheticResponse;

impl rig::completion::GetTokenUsage for SyntheticResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        None
    }
}

type Item = Result<MultiTurnStreamItem<SyntheticResponse>, anyhow::Error>;

fn tool_round(id: &str) -> Vec<Item> {
    vec![
        Ok(MultiTurnStreamItem::StreamAssistantItem(
            StreamedAssistantContent::ToolCall(ToolCall::new(
                id.to_string(),
                ToolFunction::new("Grep".to_string(), serde_json::json!({"pattern": "fn"})),
            )),
        )),
        Ok(MultiTurnStreamItem::StreamUserItem(
            StreamedUserContent::tool_result(ToolResult {
                id: id.to_string(),
                call_id: None,
                content: OneOrMany::one(ToolResultContent::text("src/lib.rs:1: fn main")),
            }),
        )),
    ]
}

#[tokio::test]
async fn sub_agent_runs_in_its_provider_specific_execution_mode() -> Result<(), String> {
    // @step When the DeepSearch sub-agent runs for a provider
    let streaming: Vec<&str> = [
        "codex",
        "zai",
        "claude",
        "openai",
        "gemini",
        "github-copilot",
    ]
    .into_iter()
    .filter(|provider| provider_uses_streaming_execution(provider))
    .collect();

    // @step Then Codex and Z.AI sub-agents run through a stream and every other provider through a single prompt call
    assert_eq!(streaming, vec!["codex", "zai"]);

    // @step And a streamed run is consumed past its tool calls until the final answer
    let mut items = tool_round("call-1");
    items.push(Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::text("thinking about the matches"),
    )));
    items.extend(tool_round("call-2"));
    items.push(Ok(MultiTurnStreamItem::final_response(
        "src/lib.rs defines main",
        rig::completion::Usage::new(),
    )));
    let answer = collect_final_response_from_stream(futures::stream::iter(items), "codex").await?;

    // @step And the final answer string is the sub-agent result
    assert_eq!(answer, "src/lib.rs defines main");

    // @step And a streamed run that ends without a final answer is an error, not an empty result
    let unfinished =
        collect_final_response_from_stream(futures::stream::iter(tool_round("call-3")), "zai")
            .await;
    let error = unfinished
        .err()
        .ok_or("a stream without a final answer must not succeed")?;
    assert!(error.contains("missing final response"), "{error}");
    Ok(())
}
