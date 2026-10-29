use super::super::output::StreamOutput;
use crate::{
    compaction_cancellation::CompactionCancellation,
    compaction_threshold::{resolve_compaction_threshold, CompactionThresholdConfig},
    interactive_helpers::convert_messages_to_turns,
    session::Session,
};
use codelet_common::token_estimator::count_tokens;
use codelet_core::{ensure_thought_signatures, RigAgent, TokenState};
use codelet_providers::selected_request::SelectedRequestPreparer;
use rig::completion::CompletionModel;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tracing::{debug, trace};

pub(super) struct Preparation {
    pub preparer: SelectedRequestPreparer,
    pub threshold: u64,
    pub context_window: u64,
    pub compaction_just_ran: bool,
}

pub(super) async fn prepare<Model: CompletionModel, Output: StreamOutput>(
    agent: &RigAgent<Model>,
    prompt: &str,
    session: &mut Session,
    output: &Output,
    session_id: uuid::Uuid,
    compaction_in_progress: &Arc<AtomicBool>,
    cancellation: &CompactionCancellation,
) -> anyhow::Result<Preparation> {
    let context_window = session.provider_manager().context_window() as u64;
    let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
    let preparer = agent
        .selected_request_preparer(
            prompt,
            codelet_providers::request_admission::AdmissionLimits {
                context_capacity: (context_window > 0).then_some(context_window),
                output_reservation: (max_output_tokens > 0).then_some(max_output_tokens),
            },
        )
        .await?;
    session.install_selected_preparer(preparer.clone());
    let model_id = session.current_model_id();
    let user_config = session
        .provider_manager()
        .compaction_threshold_override()
        .map(|(threshold_type, value)| {
            CompactionThresholdConfig::from_type_value(threshold_type, value)
        });
    let threshold = resolve_compaction_threshold(
        context_window,
        max_output_tokens,
        model_id.as_deref(),
        user_config.as_ref(),
    );
    debug!(
        "DIAG stream_loop: model={:?}, context_window={}, max_output={}, threshold={}, input_tokens={}, output_tokens={}",
        session.current_model_id(), context_window, max_output_tokens, threshold,
        session.token_tracker.input_tokens, session.token_tracker.output_tokens
    );
    let mut compaction_just_ran = false;
    let prompt_tokens = count_tokens(prompt) as u64;
    let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let has_turns_to_compact = !convert_messages_to_turns(&session.messages).is_empty();
    debug!(
        "DIAG pre-prompt check: prompt_tokens={}, current_tokens={}, estimated_total={}, threshold={}, has_turns={}, will_compact={}",
        prompt_tokens, current_tokens, estimated_total, threshold, has_turns_to_compact,
        estimated_total > threshold && has_turns_to_compact
    );
    if estimated_total > threshold && has_turns_to_compact {
        trace!(
            "Pre-prompt compaction triggered: estimated {} > threshold {}",
            estimated_total,
            threshold
        );
        output.emit_compaction_started();
        output.emit_compaction_progress("Analyzing context", 0, 0);
        let preparation_state = Arc::new(Mutex::new(TokenState::default()));
        crate::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
            session,
            session_id,
            &preparation_state,
            compaction_in_progress,
            output,
            cancellation,
        )
        .await?;
        output.emit_compaction_continuing();
        compaction_just_ran = true;
    }
    if let Some(model_id) = session.current_model_id() {
        ensure_thought_signatures(&mut session.messages, &model_id);
    } else if session.current_provider_name() == "gemini" {
        ensure_thought_signatures(&mut session.messages, "gemini-2.5-preview");
    }
    Ok(Preparation {
        preparer,
        threshold,
        context_window,
        compaction_just_ran,
    })
}
