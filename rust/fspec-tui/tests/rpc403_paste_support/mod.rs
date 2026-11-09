//! RPC-403 test support for `agent_input_paste_routing_rpc403.rs` (split
//! out by REFAC-036 to keep the test file under 300 lines): the recording
//! modal layer, the one-frame render and the RPC-411 HITL request fixture.

use std::sync::{Arc, Mutex};

use codelet_fspec_tui::{Component, EventResult, Priority};
use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::harness::AppTestHarness;

/// Minimal modal layer that records every event it receives and
/// consumes them all (modal semantics). Shared `Arc<Mutex<Vec<Event>>>`
/// lets the test inspect what the compositor actually delivered.
pub struct RecordingModal {
    pub events: Arc<Mutex<Vec<Event>>>,
}

impl Component for RecordingModal {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }
    fn id(&self) -> &str {
        "recording-modal"
    }
    fn handle_event(&mut self, event: &Event) -> EventResult {
        // No error channel here: a poisoned recorder is logged, and the
        // test's assertions on the recorded events then fail.
        match self.events.lock() {
            Ok(mut events) => events.push(event.clone()),
            Err(error) => tracing::error!("recording modal: events mutex poisoned: {error}"),
        }
        EventResult::consumed()
    }
    fn render(&mut self, _area: Rect, _buf: &mut Buffer) {}
}

/// Render the App once into an off-screen buffer so per-frame caches
/// (`AgentView::last_is_compacting` — the RPC-095 gate input) refresh.
pub fn render_once(h: &mut AppTestHarness) {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    h.app.render(area, &mut buf);
}

pub fn rpc411_hitl_request() -> codelet_rpc_types::HitlRequest {
    use codelet_rpc_types::{HitlOption, HitlQuestion, HitlRequest};
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".to_string(),
            header: "Apply?".to_string(),
            question: "Run tool?".to_string(),
            options: vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "run".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "skip".to_string(),
                },
            ],
        }],
    }
}
