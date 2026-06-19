//! Application shell + run loop (RPC-008 rules [10] [11] [12], extended
//! by RPC-009 to wire bootstrap + subscriber tasks, and by RPC-012 to
//! replace the fixed two-pane RootView with the BoardStore / AgentViewStore
//! / Navigator layout).
//!
//! Feature files:
//!   - spec/features/fspec-tui-app-shell.feature (RPC-008)
//!   - spec/features/fspec-tui-app-bootstrap-rpc009.feature (RPC-009)
//!   - spec/features/rpc012-board-agent-navigation.feature (RPC-012)
//!
//! Module layout (each child <300 LoC per RPC-012 rule [10]):
//!   - [`state`] — `App` struct, constructor, and accessors.
//!   - [`bootstrap`] — `App::bootstrap` + subscriber-task spawn helpers.
//!   - [`dispatch`] — `App::dispatch` (the single mutation surface for
//!     `BoardStore` + `AgentViewStore` per RPC-009 single-task tenere).
//!   - [`events`] — `App::handle_event` / `App::handle_paste` /
//!     `App::render` / `App::run` (terminal + crossterm + render-tick).

pub mod bootstrap;
pub mod dispatch;
pub mod dispatch_rpc018;
pub mod dispatch_rpc020;
pub mod dispatch_rpc022;
pub mod dispatch_rpc024;
pub mod dispatch_rpc025;
pub mod dispatch_rpc026;
pub mod dispatch_rpc045;
pub mod dispatch_rpc045_fspec_runner;
pub mod dispatch_rpc046;
pub mod dispatch_rpc050;
pub mod dispatch_rpc051;
pub mod dispatch_rpc052;
pub mod dispatch_rpc053;
pub mod dispatch_rpc054;
pub mod dispatch_rpc055;
pub mod dispatch_rpc056;
pub mod dispatch_rpc057;
pub mod dispatch_rpc058;
pub mod dispatch_rpc059;
pub mod dispatch_rpc060;
pub mod dispatch_rpc061;
pub mod dispatch_rpc063;
pub mod dispatch_rpc079;
pub mod dispatch_rpc098;
pub mod dispatch_rpc337;
pub mod events;
pub mod loop_parser;
pub mod schedule_parser;
pub mod slash_parser;
pub mod state;

pub use events::synth_key;
pub use slash_parser::{parse_slash_command, SlashCommandParse};
pub use state::App;

/// RPC-093 rule [6] + [11]: pure helper encoding the run-loop draw guard.
/// `should_draw <=> should_render || is_busy || is_animating`. The run
/// loop calls this each tick so:
/// - `is_busy=true` keeps the spinner ticking at 80ms cadence while the
///   session is Running/Compacting (chunk-independent redraws).
/// - `is_animating=true` keeps the Hiding/Showing finish animation
///   ticking AFTER the session has already gone Idle — without this
///   third flag the 5 char/17ms sweep-out freezes at full captured
///   text because the run loop stops drawing as soon as `is_busy`
///   becomes false.
#[must_use]
pub fn tick_should_draw(should_render: bool, is_busy: bool, is_animating: bool) -> bool {
    should_render || is_busy || is_animating
}

#[cfg(test)]
mod tick_should_draw_tests {
    use super::tick_should_draw;
    #[test]
    fn idle_no_event_skips() {
        assert!(!tick_should_draw(false, false, false));
    }
    #[test]
    fn busy_bypasses_should_render() {
        assert!(tick_should_draw(false, true, false));
    }
    #[test]
    fn event_triggers_draw() {
        assert!(tick_should_draw(true, false, false));
    }
    #[test]
    fn animating_bypasses_should_render_when_not_busy() {
        // RPC-093 fix: post-busy finish animation must keep ticking.
        assert!(tick_should_draw(false, false, true));
    }
}
