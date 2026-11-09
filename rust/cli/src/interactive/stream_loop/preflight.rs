use super::super::output::StreamOutput;
use crate::{
    compaction_cancellation::CompactionCancellation,
    compaction_threshold::{
        calculate_usable_context, compaction_threshold_for, CompactionThresholdConfig,
    },
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
    /// BUG-641: where this turn's compaction hook triggers: the threshold,
    /// or the usable window after a failed threshold compaction carried on.
    pub compaction_trigger: u64,
    pub usable_window: u64,
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
    // PROV-165: a prompt never runs against an unavailable limit; the turn
    // fails here, naming the missing limit and the model, before any request.
    let crate::model_limits::RequiredLimits {
        context_window,
        max_output_tokens,
    } = crate::model_limits::required_limits(session.provider_manager())?;
    let preparer = agent
        .selected_request_preparer(
            prompt,
            codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(context_window),
                output_reservation: Some(max_output_tokens),
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
    let threshold = compaction_threshold_for(
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
    let usable_window = calculate_usable_context(context_window, max_output_tokens);
    let mut compaction_trigger = threshold;
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
        // BUG-316: the replacement must admit the prompt it is compacting
        // for; the commit-time preparer replaces chat history, so re-append
        // the prompt (as the hook path does) and refuse a replacement that
        // cannot carry it instead of summarizing the prompt away.
        let admitted_prompt = prompt.to_owned();
        session.install_selected_preparer(preparer.clone().map_history(move |messages| {
            let mut history = messages.to_vec();
            history.push(rig::message::Message::user(admitted_prompt.clone()));
            history
        }));
        let preparation_state = Arc::new(Mutex::new(TokenState::default()));
        let round = crate::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
            session,
            session_id,
            &preparation_state,
            compaction_in_progress,
            output,
            cancellation,
        )
        .await;
        session.install_selected_preparer(preparer.clone());
        match round {
            Ok(()) => {
                output.emit_compaction_continuing();
                compaction_just_ran = true;
            }
            Err(failure) => {
                let occupied =
                    super::uncompacted::occupied_tokens(estimated_total, &session.messages, prompt);
                super::uncompacted::carry_on(session, output, failure, occupied, usable_window)?;
                compaction_trigger = usable_window;
            }
        }
    }
    if let Some(model_id) = session.current_model_id() {
        ensure_thought_signatures(&mut session.messages, &model_id);
    } else if session.current_provider_name() == "gemini" {
        ensure_thought_signatures(&mut session.messages, "gemini-2.5-preview");
    }
    Ok(Preparation {
        preparer,
        threshold,
        compaction_trigger,
        usable_window,
        context_window,
        compaction_just_ran,
    })
}
