use crate::background_session::BackgroundSession;
use codelet_rpc_types::{CommittedContextSnapshot, RequestContextSnapshot, StreamChunk};

#[derive(Default)]
pub(crate) struct ContextProjection {
    committed: Option<CommittedContextSnapshot>,
    request: Option<RequestContextSnapshot>,
}

impl BackgroundSession {
    pub(crate) fn project_committed_context(
        &self,
        projection: &mut ContextProjection,
        chunk: &StreamChunk,
    ) -> bool {
        if let StreamChunk::ContextSnapshot { snapshot } = chunk {
            if projection
                .committed
                .as_ref()
                .is_some_and(|current| current.generation > snapshot.generation)
                || projection.request.as_ref().is_some_and(|current| {
                    (current.generation, current.request) >= (snapshot.generation, snapshot.request)
                })
            {
                return false;
            }
            projection.request = Some(snapshot.clone());
            return true;
        }
        let StreamChunk::CommittedCompaction { snapshot } = chunk else {
            return true;
        };
        if projection
            .committed
            .as_ref()
            .is_some_and(|current| current.generation >= snapshot.generation)
            || projection
                .request
                .as_ref()
                .is_some_and(|current| current.generation > snapshot.generation)
        {
            return false;
        }
        if projection
            .request
            .as_ref()
            .is_none_or(|current| current.generation != snapshot.generation)
        {
            self.update_tokens(snapshot.tokens.input_tokens, snapshot.tokens.output_tokens);
            self.update_reasoning_tokens(snapshot.tokens.reasoning_tokens.unwrap_or(0));
        }
        projection.committed = Some(snapshot.clone());
        true
    }

    pub fn committed_context_snapshot(&self) -> Option<CommittedContextSnapshot> {
        match self.committed_context.lock() {
            Ok(projection) => projection.committed.clone(),
            Err(error) => {
                tracing::error!(%error, "committed context projection lock poisoned");
                None
            }
        }
    }
}
