//! CMPCT-049 — `StreamChunk::CompactionComplete` arm body.
//!
//! Feature: spec/features/compaction-diagnostic-log-level-hygiene.feature
//!
//! Factored out of `dispatch_stream_chunks.rs` unchanged so the caller
//! stays under the 300-LoC ceiling pinned by `source_shape_rpc049.rs` /
//! `source_shape_rpc050.rs`. The `CompactionComplete` arm of
//! `App::handle_stream_chunk_state_updates` now delegates to
//! [`App::apply_compaction_complete`].
//!
//! RPC-421: this file is the SINGLE source of the compaction success
//! notice — it fires for both `/compact` and auto-compaction and carries
//! the honest post-injection numbers (CMPCT-038 apply-site), so exactly
//! one `[compaction]` notice lands per compaction.
//!
//! RPC-417: the auto-hide timer arming lives in `dispatch_compaction_hide.rs`
//! (same spawn → sleep → self-addressed Action + seq-guard pattern).

use codelet_rpc_types::{CompactionResult, SessionId};

use crate::components::Action;

use super::state::App;

impl App {
    /// Apply the `StreamChunk::CompactionComplete` side effects:
    ///
    /// 1. Clear the per-session compaction-progress entry.
    /// 2. Persist the reduction percentage on the per-session slot so
    ///    SessionHeader renders the `[X%: COMPACTED Y%]` badge suffix.
    ///    RPC-420: the wire `compression_ratio` is already the PERCENT of
    ///    tokens removed [0,100], so it is rounded and displayed
    ///    directly — same convention as `format_compaction_notice`
    ///    below, keeping the notice line and the badge in sync.
    ///    CMPCT-040: clamp at this single writer (`.max(0.0)`) so a
    ///    negative wire value from a stale/unclamped backend can never
    ///    reach the store — the header renders the stored value verbatim
    ///    and must never sign-flip growth into a fake positive reduction.
    /// 3. RPC-417: arm the 10-second per-session auto-hide timer
    ///    (TS TUI-044 parity). Bump the seq first so a stale fire from an
    ///    earlier compaction becomes a no-op, then arm (runtime-guarded
    ///    → no-op under a synchronous #[test]).
    /// 4. RPC-421: emit the single user-facing notice via
    ///    `Action::EmitSessionNotice` so the `[compaction] ...` line lands
    ///    in the originating session's scrollback regardless of focus.
    pub(crate) fn apply_compaction_complete(
        &mut self,
        session_id: &SessionId,
        result: &CompactionResult,
    ) {
        self.agent_view_store.clear_compaction_progress(session_id);

        let reduction = result.compression_ratio.round().max(0.0) as i32;
        self.agent_view_store
            .set_compaction_reduction(session_id.clone(), reduction);

        let seq = self
            .agent_view_store
            .bump_compaction_reduction_seq(session_id.clone());
        self.arm_compaction_hide(session_id.clone(), seq);

        let text = format_compaction_notice(result);
        let _ = self
            .action_tx
            .send(Action::EmitSessionNotice(session_id.clone(), text));
    }
}

/// RPC-047: format a `CompactionResult` into the user-facing scrollback
/// notice line.
///
/// RPC-421: its SOLE caller is the `StreamChunk::CompactionComplete`
/// handler — the `/compact` Ok branch no longer emits a notice from the
/// RPC result (whose numbers are measured before DAG injection). Exactly
/// one `[compaction]` line lands per compaction, carrying the honest
/// post-injection numbers.
///
/// Example output:
/// ```text
/// [compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)
/// ```
///
/// RPC-420: `compression_ratio` is already the PERCENT of tokens removed
/// [0,100]; render it directly — never `(1.0 - ratio) * 100.0`.
pub(crate) fn format_compaction_notice(result: &CompactionResult) -> String {
    let reduction_pct = result.compression_ratio;
    format!(
        "[compaction] {reduction:.1}% reduction ({orig} \u{2192} {compacted} tokens, {turns} turns summarised)",
        reduction = reduction_pct,
        orig = result.original_tokens,
        compacted = result.compacted_tokens,
        turns = result.turns_summarized,
    )
}
