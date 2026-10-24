use super::output::{ContextFillInfo, StreamEvent, StreamOutput};
use codelet_rpc_types::{ContextProvenance, RequestContextSnapshot};
use rig::completion::{RequestContext, Usage};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static CONTEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) struct ContextProjection {
    pub session_id: Uuid,
    pub model: String,
    pub capacity: u64,
    pub threshold: u64,
}

impl ContextProjection {
    pub fn emit<O: StreamOutput>(
        &self,
        output: &O,
        context: Option<&RequestContext>,
        usage: Option<Usage>,
    ) {
        let generation = codelet_core::persistence::load_session(self.session_id)
            .ok()
            .map(|manifest| {
                manifest
                    .compaction_commit
                    .map_or(0, |commit| commit.identity.generation)
            });
        let generation = context
            .filter(|context| !context.model.is_empty())
            .map(|context| context.generation)
            .or(generation);
        let model = context
            .filter(|context| !context.model.is_empty())
            .map_or_else(|| self.model.clone(), |context| context.model.clone());
        let usage = usage.filter(|_| generation.is_some());
        let snapshot = RequestContextSnapshot {
            model,
            generation: generation
                .and_then(|value| i64::try_from(value).ok())
                .unwrap_or(-1),
            request: CONTEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            physical_request: context.map(|context| context.request.clone()),
            input_tokens: usage.map(|usage| {
                usage
                    .input_tokens
                    .saturating_add(usage.cache_read_input_tokens.unwrap_or(0))
                    .saturating_add(usage.cache_creation_input_tokens.unwrap_or(0))
            }),
            output_tokens: usage.map(|usage| {
                usage
                    .output_tokens
                    .saturating_add(usage.reasoning_tokens.unwrap_or(0))
            }),
            capacity: (self.capacity > 0).then_some(self.capacity),
            threshold: (self.threshold > 0).then_some(self.threshold),
            provenance: if usage.is_some() {
                ContextProvenance::Measured
            } else {
                ContextProvenance::Unknown
            },
        };
        if let Some(fill) = snapshot.context_fill() {
            output.emit_context_fill(&ContextFillInfo {
                fill_percentage: fill.fill_percentage,
                effective_tokens: fill.effective_tokens as u64,
                threshold: self.threshold,
                context_window: self.capacity,
            });
        }
        output.emit(StreamEvent::ContextSnapshot(snapshot));
    }
}
