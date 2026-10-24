use super::super::message_helpers::add_assistant_tool_calls_message;
use super::super::output::StreamOutput;
use super::super::recovery_compaction::{begin_compaction_recovery, CompactionRecoveryPolicy};
use super::super::stream_handlers::handle_final_response;
use crate::session::Session;
use anyhow::Result;
use codelet_core::{ApiTokenUsage, StreamingTokenDisplay, TokenState};
use rig::agent::FinalResponse;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub(super) struct ContinuationState {
    pub text: String,
    pub tool_calls_buffer: Vec<rig::message::AssistantContent>,
    pub last_tool_name: Option<String>,
    pub display: StreamingTokenDisplay,
    pub token_state: Arc<Mutex<TokenState>>,
    pub tool_calls: usize,
    finalized_output: u64,
    finalized_reasoning: u64,
}

impl ContinuationState {
    pub fn new(display: StreamingTokenDisplay, token_state: Arc<Mutex<TokenState>>) -> Self {
        let current = display.current();
        Self {
            text: String::new(),
            tool_calls_buffer: Vec::new(),
            last_tool_name: None,
            display,
            token_state,
            tool_calls: 0,
            finalized_output: current.output_tokens,
            finalized_reasoning: current.reasoning_tokens,
        }
    }

    pub fn finalize(&mut self, response: &FinalResponse) {
        let usage = response.usage();
        let physical = response
            .request_context()
            .map_or(usage, |context| context.usage.unwrap_or_default());
        if physical.input_tokens > 0
            || physical.cache_read_input_tokens.unwrap_or(0) > 0
            || physical.cache_creation_input_tokens.unwrap_or(0) > 0
            || usage.output_tokens > 0
            || usage.reasoning_tokens.unwrap_or(0) > 0
        {
            let display_usage = rig::completion::Usage {
                input_tokens: physical.input_tokens,
                cache_read_input_tokens: physical.cache_read_input_tokens,
                cache_creation_input_tokens: physical.cache_creation_input_tokens,
                ..usage
            };
            self.display = StreamingTokenDisplay::new(
                physical.input_tokens,
                self.finalized_output,
                physical.cache_read_input_tokens.unwrap_or(0),
                physical.cache_creation_input_tokens.unwrap_or(0),
            )
            .with_prev_reasoning(self.finalized_reasoning);
            self.display.update_from_final_response(&display_usage);
        }
        let current = self.display.current();
        self.finalized_output = current.output_tokens;
        self.finalized_reasoning = current.reasoning_tokens;
    }

    pub fn needs_recovery(&self, parent: &Arc<Mutex<TokenState>>) -> bool {
        let local_needed = self
            .token_state
            .lock()
            .map(|state| state.compaction_needed)
            .unwrap_or(false);
        if let Ok(mut state) = parent.lock() {
            state.compaction_needed |= local_needed;
            state.compaction_needed
        } else {
            local_needed
        }
    }

    pub fn recover<O: StreamOutput>(
        &mut self,
        session: &mut Session,
        parent: &Arc<Mutex<TokenState>>,
        output: &O,
    ) -> Result<CompactionRecoveryPolicy> {
        self.tool_calls_buffer.retain(|buffered| {
            let rig::message::AssistantContent::ToolCall(buffered) = buffered else {
                return true;
            };
            !session.messages.iter().any(|message| {
                matches!(message, rig::message::Message::Assistant { content, .. }
                    if content.iter().any(|content| matches!(content,
                        rig::message::AssistantContent::ToolCall(existing)
                            if existing.id == buffered.id && existing.call_id == buffered.call_id)))
            })
        });
        if !self.tool_calls_buffer.is_empty() {
            add_assistant_tool_calls_message(
                &mut session.messages,
                std::mem::take(&mut self.tool_calls_buffer),
            )?;
        }
        begin_compaction_recovery(
            session,
            parent,
            &self.display,
            &mut self.text,
            output,
            false,
        )
    }

    pub fn finish_history(&mut self, session: &mut Session, session_id: Uuid) -> Result<()> {
        if !self.text.is_empty() {
            handle_final_response(&self.text, &mut session.messages)?;
            self.text.clear();
        }
        let current = self.display.current();
        let usage = ApiTokenUsage::new(
            current.input_tokens,
            current.cache_read_tokens,
            current.cache_creation_tokens,
            0,
        )
        .with_reasoning_tokens(current.reasoning_tokens);
        session
            .token_tracker
            .update_display_only(&usage, current.output_tokens);
        let settled =
            codelet_core::persistence::load_session(session_id).map_err(anyhow::Error::msg)?;
        session.token_tracker.cumulative_billed_input = settled.token_usage.cumulative_billed_input;
        session.token_tracker.cumulative_billed_output =
            settled.token_usage.cumulative_billed_output;
        Ok(())
    }

    pub fn start_nested(&mut self) {
        let current = self.display.current();
        self.token_state = Arc::new(Mutex::new(TokenState {
            input_tokens: current.input_tokens,
            cache_read_input_tokens: current.cache_read_tokens,
            cache_creation_input_tokens: current.cache_creation_tokens,
            output_tokens: current.output_tokens,
            compaction_needed: false,
        }));
        self.display = StreamingTokenDisplay::new(
            current.input_tokens,
            current.output_tokens,
            current.cache_read_tokens,
            current.cache_creation_tokens,
        )
        .with_prev_reasoning(current.reasoning_tokens);
        self.tool_calls_buffer.clear();
        self.last_tool_name = None;
    }
}
