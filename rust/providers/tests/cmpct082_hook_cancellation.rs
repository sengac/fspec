use codelet_core::RigAgent;
use futures::StreamExt;
use rig::agent::{AgentBuilder, CancelSignal, MultiTurnStreamItem, StreamingPromptHook};
use rig::completion::Message;
use rig::providers::anthropic::{
    completion::CompletionModel, streaming::StreamingCompletionResponse,
};

#[path = "fixtures/cmpct069_anthropic.rs"]
mod fixture;

#[derive(Clone)]
struct Guard(&'static str);

impl StreamingPromptHook<CompletionModel> for Guard {
    async fn on_completion_call(&self, _: &Message, _: &[Message], signal: CancelSignal) {
        if self.0 == "before request" {
            signal.cancel();
        }
    }

    async fn on_text_delta(&self, _: &str, _: &str, signal: CancelSignal) {
        if self.0 == "during text" {
            signal.cancel();
        }
    }

    async fn on_stream_completion_response_finish(
        &self,
        _: &Message,
        _: &StreamingCompletionResponse,
        signal: CancelSignal,
    ) {
        if self.0 == "at finish" {
            signal.cancel();
        }
    }
}

#[tokio::test]
async fn a_cancelled_streaming_guard_cannot_become_successful_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    for phase in ["before request", "during text", "at finish"] {
        // @step Given an ordinary provider guard cancels <phase>
        let (server, model) =
            fixture::model_with_expected_requests(u64::from(phase != "before request")).await?;
        let agent = RigAgent::with_default_depth(AgentBuilder::new(model).max_tokens(200).build());
        let mut history = vec![];
        // @step When the real streaming agent consumes the provider response
        let stream = agent
            .prompt_streaming_with_history_and_hook("hello", &mut history, Guard(phase))
            .await;
        futures::pin_mut!(stream);
        let mut failures = 0;
        let mut completions = 0;
        while let Some(event) = stream.next().await {
            match event {
                Err(_) => failures += 1,
                Ok(MultiTurnStreamItem::FinalResponse(_)) => completions += 1,
                _ => {}
            }
        }
        // @step Then cancellation returns an explicit failure without successful completion
        assert_eq!(failures, 1, "{phase}");
        assert_eq!(completions, 0, "{phase}");
        // @step And a before-request cancellation sends no physical inference
        if phase == "before request" {
            assert!(server
                .received_requests()
                .await
                .ok_or("capture missing")?
                .is_empty());
        }
    }
    Ok(())
}
