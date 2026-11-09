//! Feature: spec/features/a-panic-in-one-session-never-stops-other-sessions-persistence.feature
//!
//! BUG-610: the OpenAI Responses conversion (rig responses_api, used by the
//! Codex model) called `expect` on tool call ids, so a history item without
//! one panicked instead of failing the request.

use codelet_providers::codex::CodexResponsesModel;
use codelet_providers::CodexProvider;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{AssistantContent, Message, ToolCall, ToolFunction};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MODEL: &str = "gpt-6.1-sol";

fn convert(history: Vec<Message>) -> Result<String, Box<dyn std::error::Error>> {
    let provider = CodexProvider::from_api_key("synthetic-offline-key", MODEL)?;
    let model = CodexResponsesModel::new(provider.client().clone(), MODEL);
    let mut request = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    request.chat_history = rig::OneOrMany::many(history)?;
    match model.prepare_request(request) {
        Ok(_) => Err("a history item without a call id must not convert".into()),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn a_tool_call_or_tool_result_without_a_call_id_is_a_request_error_not_a_panic() -> TestResult {
    // @step Given a request whose history holds a tool result with no call id
    let result_history = vec![
        Message::user("run it"),
        Message::tool_result_with_call_id("fc_result", None, "output"),
    ];

    // @step When the request is converted for the OpenAI Responses API
    let converted = std::panic::catch_unwind(|| convert(result_history))
        .map_err(|_| "the conversion panicked")?;

    // @step Then the conversion fails with "tool result <id> has no call id"
    let error = converted?;
    assert!(
        error.contains("tool result fc_result has no call id"),
        "{error}"
    );

    // @step And a request whose history holds a tool call with no call id fails with "tool call <id> has no call id"
    let call_history = vec![
        Message::user("run it"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                id: "fc_call".into(),
                call_id: None,
                function: ToolFunction {
                    name: "shell_command".into(),
                    arguments: json!({"command": "ls"}),
                },
                signature: None,
                additional_params: None,
            })),
        },
    ];
    let error = std::panic::catch_unwind(|| convert(call_history))
        .map_err(|_| "the conversion panicked")??;
    assert!(
        error.contains("tool call fc_call has no call id"),
        "{error}"
    );
    Ok(())
}
