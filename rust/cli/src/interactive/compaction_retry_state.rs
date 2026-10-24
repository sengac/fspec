use codelet_core::{CompactionHook, TokenState};
use std::sync::{Arc, Mutex};

pub fn retry_compaction_hook(
    state: &Arc<Mutex<TokenState>>,
    input_tokens: u64,
    output_tokens: u64,
    threshold: u64,
) -> CompactionHook {
    match state.lock() {
        Ok(mut current) => {
            let compaction_needed = current.compaction_needed;
            *current = TokenState::from_cache_inclusive_total(input_tokens, output_tokens);
            current.compaction_needed = compaction_needed;
        }
        Err(error) => {
            tracing::error!(%error, "Retry compaction state unavailable; hook will cancel")
        }
    }
    CompactionHook::new(state.clone(), threshold)
}
