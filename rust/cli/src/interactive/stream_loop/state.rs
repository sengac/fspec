use super::super::{context_projection::ContextProjection, output::StreamOutput};
use crate::{compaction_cancellation::CompactionCancellation, session::Session};
use codelet_core::compaction::annotation_detector::ToolCallInfo;
use codelet_core::{compaction::InputUsageObservation, StreamingTokenDisplay, TokenState};
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_tui::InputQueue;
use rig::{agent::MultiTurnStreamItem, completion::CompletionModel, message::AssistantContent};
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};
use tokio::sync::Notify;

pub(super) type AgentStream<'agent, Model> = futures::stream::BoxStream<
    'agent,
    Result<MultiTurnStreamItem<<Model as CompletionModel>::StreamingResponse>, anyhow::Error>,
>;

pub(super) enum Control {
    Next,
    Continue,
    Stop,
}

pub(super) struct Context<'call, Output> {
    pub session: &'call mut Session,
    pub prompt: &'call str,
    pub output: &'call Output,
    pub session_id: uuid::Uuid,
    pub is_interrupted: Arc<AtomicBool>,
    pub compaction_in_progress: Arc<AtomicBool>,
    pub interrupt_notify: Option<Arc<Notify>>,
    pub compaction_cancellation: CompactionCancellation,
    pub input_queue: Option<&'call mut InputQueue>,
    pub cli_mode: bool,
    pub request_id: String,
    pub api_start_time: Instant,
    pub preparer: SelectedRequestPreparer,
    pub threshold: u64,
    pub context_window: u64,
    pub token_state: Arc<Mutex<TokenState>>,
    pub active_tool_call_id: Arc<Mutex<Option<String>>>,
    pub request_history_len: usize,
    pub context_projection: ContextProjection,
}

pub(super) struct State {
    pub assistant_text: String,
    pub accumulated_reasoning: String,
    pub response_attempt_snapshot: Option<(usize, usize, StreamingTokenDisplay)>,
    pub input_observation: InputUsageObservation,
    pub final_stop_reason: Option<String>,
    pub truncation_retry_count: u32,
    pub thinking_exhaustion_retry_count: u32,
    pub network_retry_count: u32,
    pub compaction_retry_count: u32,
    pub network_retry_in_progress: bool,
    pub tool_calls_buffer: Vec<AssistantContent>,
    pub last_tool_name: Option<String>,
    pub summary_handoff: bool,
    pub tool_execution_in_progress: bool,
    pub turn_tool_infos: Vec<ToolCallInfo>,
    pub previous_turn_tool_infos: Vec<ToolCallInfo>,
    pub auto_continue_segment_tool_calls: usize,
    pub auto_continue_nudge_pending_refund: bool,
    pub goal_consecutive_zero_activity_nudges: u32,
    pub prev_reasoning: u64,
    pub finalized_output: u64,
    pub finalized_reasoning: u64,
    pub streaming_display: StreamingTokenDisplay,
}

impl State {
    pub fn new<Output: StreamOutput>(session: &Session, output: &Output) -> Self {
        let tracker = &session.token_tracker;
        let prev_input_tokens = tracker.input_tokens;
        let prev_output_tokens = tracker.output_tokens;
        let prev_cache_read = tracker.cache_read_input_tokens.unwrap_or(0);
        let prev_cache_creation = tracker.cache_creation_input_tokens.unwrap_or(0);
        let prev_reasoning = tracker.reasoning_tokens;
        let streaming_display = StreamingTokenDisplay::from_cache_inclusive_total(
            prev_input_tokens,
            prev_output_tokens,
            prev_cache_read,
            prev_cache_creation,
        )
        .with_prev_reasoning(prev_reasoning);
        tracing::trace!(
            "Initial token emit: prev_input_tokens={}, prev_output_tokens={}, cache_read={}, cache_creation={}",
            prev_input_tokens, prev_output_tokens, prev_cache_read, prev_cache_creation
        );
        output.emit_tokens(&streaming_display.current().into());
        Self {
            assistant_text: String::new(),
            accumulated_reasoning: String::new(),
            response_attempt_snapshot: None,
            input_observation: Default::default(),
            final_stop_reason: None,
            truncation_retry_count: 0,
            thinking_exhaustion_retry_count: 0,
            network_retry_count: 0,
            compaction_retry_count: 0,
            network_retry_in_progress: false,
            tool_calls_buffer: Vec::new(),
            last_tool_name: None,
            summary_handoff: false,
            tool_execution_in_progress: false,
            turn_tool_infos: Vec::new(),
            previous_turn_tool_infos: Vec::new(),
            auto_continue_segment_tool_calls: 0,
            auto_continue_nudge_pending_refund: false,
            goal_consecutive_zero_activity_nudges: 0,
            prev_reasoning,
            finalized_output: prev_output_tokens,
            finalized_reasoning: prev_reasoning,
            streaming_display,
        }
    }

    pub fn received<Output: StreamOutput>(&mut self, output: &Output) {
        if self.network_retry_in_progress {
            output.emit_status("✓ Reconnected");
            self.network_retry_in_progress = false;
        }
        self.network_retry_count = 0;
    }
}
