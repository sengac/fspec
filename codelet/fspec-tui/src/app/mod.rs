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
pub mod dispatch_rpc024;
pub mod dispatch_rpc025;
pub mod dispatch_rpc026;
pub mod events;
pub mod state;

pub use events::synth_key;
pub use state::App;
