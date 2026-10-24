use crate::compaction_threshold::{resolve_compaction_threshold, CompactionThresholdConfig};
use crate::interactive::StreamOutput;
use crate::session::Session;
use codelet_core::persistence::compaction_boundary::{
    CommittedGeneration, CompactionTokenProvenance,
};
use codelet_core::persistence::load_session;
use codelet_rpc_types::{
    CommittedContextSnapshot, ContextProvenance, RequestContextSnapshot, TokenTracker,
};

pub(crate) struct CompactionPublication {
    original_tokens: u64,
    capacity: u64,
    threshold: u64,
}

impl CompactionPublication {
    pub fn new(session: &Session) -> Self {
        let manager = session.provider_manager();
        let capacity = manager.context_window() as u64;
        let reservation = manager.max_output_tokens() as u64;
        let configuration = manager
            .compaction_threshold_override()
            .map(|(kind, value)| CompactionThresholdConfig::from_type_value(kind, value));
        Self {
            original_tokens: session.token_tracker.input_tokens,
            capacity,
            threshold: resolve_compaction_threshold(
                capacity,
                reservation,
                session.current_model_id().as_deref(),
                configuration.as_ref(),
            ),
        }
    }

    pub fn publish<O: StreamOutput>(
        &self,
        output: &O,
        boundary: &CommittedGeneration,
    ) -> Result<(), String> {
        let manifest = load_session(boundary.identity.session_id)?;
        if manifest.compaction_commit.as_ref() != Some(boundary) {
            return Err("committed publication identity changed".into());
        }
        let convert = |value| {
            u32::try_from(value)
                .map_err(|_| "committed publication exceeds token representation".to_owned())
        };
        let generation =
            i64::try_from(boundary.identity.generation).map_err(|error| error.to_string())?;
        let provenance = match boundary.token_provenance {
            CompactionTokenProvenance::Estimated => ContextProvenance::Estimated,
            CompactionTokenProvenance::ProviderMeasured => ContextProvenance::Measured,
        };
        let context = RequestContextSnapshot {
            model: boundary.checkpoint.model.clone(),
            generation,
            request: 0,
            physical_request: None,
            input_tokens: Some(boundary.input_tokens),
            output_tokens: Some(0),
            capacity: Some(self.capacity),
            threshold: Some(self.threshold),
            provenance,
        };
        let fill = context
            .context_fill()
            .ok_or("committed context requires usable selected limits")?;
        let saved = &manifest.token_usage;
        let snapshot = CommittedContextSnapshot {
            generation,
            tokens: TokenTracker {
                input_tokens: convert(boundary.input_tokens)?,
                output_tokens: convert(saved.cumulative_billed_output)?,
                cache_read_input_tokens: Some(convert(saved.cache_read_tokens)?),
                cache_creation_input_tokens: Some(convert(saved.cache_creation_tokens)?),
                cumulative_billed_input: Some(convert(saved.cumulative_billed_input)?),
                cumulative_billed_output: Some(convert(saved.cumulative_billed_output)?),
                reasoning_tokens: Some(convert(saved.reasoning_tokens)?),
                tokens_per_second: None,
            },
            context_fill: fill,
            original_tokens: convert(self.original_tokens)?,
            covered_turns: Some(
                u32::try_from(boundary.covered_prefix.len()).map_err(|error| error.to_string())?,
            ),
            retained_turns: Some(
                u32::try_from(boundary.verified_suffix.len()).map_err(|error| error.to_string())?,
            ),
            estimated: provenance == ContextProvenance::Estimated,
            reconciled: true,
        };
        output.publish_committed_context(&boundary.identity, snapshot, context)
    }
}
