//! RPC-094 — shared scrollback dispatch helper, split out of `dispatch.rs`
//! so that file stays under the 300-LoC source-shape ceiling while keeping
//! canonical rustfmt formatting.

use super::state::App;

impl App {
    /// RPC-094: shared scrollback dispatch helper.
    pub(crate) fn scroll_focused(&mut self, delta: i64) {
        if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
            if delta < 0 {
                ctx.scrollback.scroll_up(delta.unsigned_abs() as usize);
            } else if delta > 0 {
                ctx.scrollback.scroll_down(delta as usize);
            }
        }
    }
}
