//! AMGR-022 — fold `StreamChunk::InteractionRequestChanged` into the
//! store's chain requests.
//!
//! Feature: spec/features/a-sub-agents-request-shows-in-the-root-view.feature
//!
//! A Pending event that hands a sub-agent's request to the user reads the
//! origin's question or approval (the RPC-053 probe on the origin), so the
//! root view can draw it; an Answered or Cancelled event drops it from the
//! root view, the origin's tab and every waiting marker.

use codelet_rpc_types::{RequestRoute, RequestStatus};

use super::state::App;

impl App {
    pub(crate) fn handle_request_changed(&mut self, route: &RequestRoute, status: &RequestStatus) {
        if self.agent_view_store.apply_request_change(route, status) {
            self.handle_pause_chunk(route.origin.clone());
        }
        self.should_render = true;
    }
}
