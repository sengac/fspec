#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/multi-turn-tool-continuation.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! CONT-001: rig-core streaming loop exits with unanswered tool results when
//! stop_reason=stop. The multi-turn exit condition in
//! src/agent/prompt_request/streaming.rs (~line 802) is purely `if !did_call_tool`,
//! and trailing Text/Reasoning chunks after a ToolCall reset did_call_tool=false,
//! so a turn that executed tools but ended with trailing text exits the loop
//! without feeding tool results back to the model.
//!
//! Harness: a scripted CompletionModel that plays back per-turn queues of
//! RawStreamingChoice values and records every CompletionRequest it receives.

use std::sync::{Arc, Mutex};

use futures::StreamExt;
use rig::agent::{AgentBuilder, MultiTurnStreamItem};
use rig::completion::{CompletionError, CompletionRequest, CompletionResponse, GetTokenUsage};
use rig::message::AssistantContent;
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse, StreamingPrompt,
};
use serde::{Deserialize, Serialize};

/// Streaming response payload carrying the scripted stop_reason (PROV-039 path).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScriptedResponse {
    stop_reason: Option<String>,
}

impl GetTokenUsage for ScriptedResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        let mut usage = rig::completion::Usage::new();
        usage.input_tokens = 1;
        usage.output_tokens = 1;
        usage.total_tokens = 2;
        Some(usage)
    }

    fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }
}

/// One scripted turn: the chunks the model will stream for that request.
type Turn = Vec<RawStreamingChoice<ScriptedResponse>>;

/// A scripted CompletionModel that records every request and plays back
/// pre-canned streaming turns.
#[derive(Clone)]
struct ScriptedModel {
    turns: Arc<Mutex<Vec<Turn>>>,
    requests: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl ScriptedModel {
    fn new(turns: Vec<Turn>) -> Self {
        Self {
            turns: Arc::new(Mutex::new(turns)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    fn request(&self, idx: usize) -> CompletionRequest {
        let requests = self.requests.lock().unwrap();
        CompletionRequest {
            preamble: requests[idx].preamble.clone(),
            chat_history: requests[idx].chat_history.clone(),
            documents: requests[idx].documents.clone(),
            tools: requests[idx].tools.clone(),
            temperature: requests[idx].temperature,
            max_tokens: requests[idx].max_tokens,
            tool_choice: requests[idx].tool_choice.clone(),
            additional_params: requests[idx].additional_params.clone(),
        }
    }
}

impl rig::completion::CompletionModel for ScriptedModel {
    type Response = ScriptedResponse;
    type StreamingResponse = ScriptedResponse;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
        ScriptedModel::new(vec![])
    }

    async fn completion(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        Err(CompletionError::ProviderError(
            "non-streaming completion not scripted".to_string(),
        ))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        self.requests.lock().unwrap().push(request);
        let mut turns = self.turns.lock().unwrap();
        if turns.is_empty() {
            return Err(CompletionError::ProviderError(
                "scripted model ran out of turns".to_string(),
            ));
        }
        let turn = turns.remove(0);
        let stream = futures::stream::iter(turn.into_iter().map(Ok));
        let pinned: rig::streaming::StreamingResult<ScriptedResponse> = Box::pin(stream);
        Ok(StreamingCompletionResponse::stream(pinned))
    }
}

/// Chunk builders
fn text(t: &str) -> RawStreamingChoice<ScriptedResponse> {
    RawStreamingChoice::Message(t.to_string())
}

fn tool_call(id: &str, name: &str) -> RawStreamingChoice<ScriptedResponse> {
    RawStreamingChoice::ToolCall(
        RawStreamingToolCall::new(
            id.to_string(),
            name.to_string(),
            serde_json::json!({"input": "x"}),
        )
        .with_call_id(id.to_string()),
    )
}

fn reasoning_delta(r: &str) -> RawStreamingChoice<ScriptedResponse> {
    RawStreamingChoice::ReasoningDelta {
        id: None,
        reasoning: r.to_string(),
    }
}

fn final_with_stop(reason: &str) -> RawStreamingChoice<ScriptedResponse> {
    RawStreamingChoice::FinalResponse(ScriptedResponse {
        stop_reason: Some(reason.to_string()),
    })
}

/// Drive the multi-turn stream to completion, returning the FinalResponse text.
async fn drive(model: &ScriptedModel) -> Option<String> {
    let agent = AgentBuilder::new(model.clone()).build();
    let mut stream = agent.stream_prompt("do the task").multi_turn(5).await;
    let mut final_text: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::FinalResponse(res)) => {
                final_text = Some(res.response().to_string());
            }
            Ok(_) => {}
            Err(e) => panic!("stream error: {e}"),
        }
    }
    final_text
}

/// Does this request's chat history contain a user tool_result message?
fn history_has_tool_result(request: &CompletionRequest) -> bool {
    request.chat_history.iter().any(|msg| match msg {
        rig::message::Message::User { content } => content.iter().any(|c| {
            matches!(c, rig::message::UserContent::ToolResult(_))
        }),
        _ => false,
    })
}

/// Does this request's chat history contain an assistant tool_call message?
fn history_has_tool_call(request: &CompletionRequest) -> bool {
    request.chat_history.iter().any(|msg| match msg {
        rig::message::Message::Assistant { content, .. } => content
            .iter()
            .any(|c| matches!(c, AssistantContent::ToolCall(_))),
        _ => false,
    })
}

// =========================================================================
// Scenario: Turn with a tool call and trailing text before stop_reason
// end_turn continues to a second request
// =========================================================================
#[tokio::test]
async fn test_tool_call_with_trailing_text_end_turn_continues() {
    // @step Given a scripted model whose first turn streams a tool call, then trailing text, then a final response with stop_reason "end_turn"
    let turn1 = vec![
        tool_call("call-1", "lookup"),
        text("let me summarise..."),
        final_with_stop("end_turn"),
    ];
    // @step And the scripted model's second turn streams only text "final answer" with stop_reason "end_turn"
    let turn2 = vec![text("final answer"), final_with_stop("end_turn")];
    let model = ScriptedModel::new(vec![turn1, turn2]);

    // @step When the agent streams the prompt with multi-turn enabled
    let final_text = drive(&model).await;

    // @step Then the model receives a second completion request
    assert_eq!(
        model.request_count(),
        2,
        "BUG CONT-001: loop exited after turn 1 (trailing text reset did_call_tool) — \
         tool results were never sent back to the model"
    );

    // @step And the second request's chat history contains the tool result answering the first turn's tool call
    let second = model.request(1);
    assert!(
        history_has_tool_call(&second),
        "second request must carry the assistant tool_call from turn 1"
    );
    assert!(
        history_has_tool_result(&second),
        "second request must carry the tool_result answering turn 1's tool call"
    );

    // @step And the final response text is "final answer"
    assert_eq!(final_text.as_deref(), Some("final answer"));
}

// =========================================================================
// Scenario: Turn with a tool call and a trailing reasoning delta continues
// to a second request
// =========================================================================
#[tokio::test]
async fn test_tool_call_with_trailing_reasoning_delta_continues() {
    // @step Given a scripted model whose first turn streams a tool call, then a trailing reasoning delta, then a final response with stop_reason "end_turn"
    let turn1 = vec![
        tool_call("call-2", "lookup"),
        reasoning_delta("thinking about the result..."),
        final_with_stop("end_turn"),
    ];
    // @step And the scripted model's second turn streams only text "done after reasoning" with stop_reason "end_turn"
    let turn2 = vec![text("done after reasoning"), final_with_stop("end_turn")];
    let model = ScriptedModel::new(vec![turn1, turn2]);

    // @step When the agent streams the prompt with multi-turn enabled
    let final_text = drive(&model).await;

    // @step Then the model receives a second completion request
    assert_eq!(
        model.request_count(),
        2,
        "BUG CONT-001: loop exited after turn 1 (trailing ReasoningDelta reset did_call_tool)"
    );

    // @step And the final response text is "done after reasoning"
    assert_eq!(final_text.as_deref(), Some("done after reasoning"));
}

// =========================================================================
// Scenario: Text-only turn exits the loop after exactly one request
// =========================================================================
#[tokio::test]
async fn test_text_only_turn_exits_after_one_request() {
    // @step Given a scripted model whose only turn streams text "plain answer" and a final response with stop_reason "end_turn"
    let turn1 = vec![text("plain answer"), final_with_stop("end_turn")];
    let model = ScriptedModel::new(vec![turn1]);

    // @step When the agent streams the prompt with multi-turn enabled
    let final_text = drive(&model).await;

    // @step Then the model receives exactly one completion request
    assert_eq!(
        model.request_count(),
        1,
        "text-only turn must NOT continue the loop (no spurious continuation)"
    );

    // @step And the final response text is "plain answer"
    assert_eq!(final_text.as_deref(), Some("plain answer"));
}

// =========================================================================
// Scenario: Tool-only turn with stop_reason tool_use still continues to a
// second request
// =========================================================================
#[tokio::test]
async fn test_tool_only_turn_tool_use_still_continues() {
    // @step Given a scripted model whose first turn streams a tool call then a final response with stop_reason "tool_use" and no trailing chunks
    let turn1 = vec![tool_call("call-3", "lookup"), final_with_stop("tool_use")];
    // @step And the scripted model's second turn streams only text "after tool" with stop_reason "end_turn"
    let turn2 = vec![text("after tool"), final_with_stop("end_turn")];
    let model = ScriptedModel::new(vec![turn1, turn2]);

    // @step When the agent streams the prompt with multi-turn enabled
    let final_text = drive(&model).await;

    // @step Then the model receives a second completion request
    assert_eq!(
        model.request_count(),
        2,
        "regression guard: tool-only turn must continue the loop as it does today"
    );

    // @step And the final response text is "after tool"
    assert_eq!(final_text.as_deref(), Some("after tool"));
}
