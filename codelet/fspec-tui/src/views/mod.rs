//! View layer.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc012-board-store.feature
//!   - spec/features/rpc013-board-footer.feature
//!   - spec/features/rpc013-agent-footer.feature
//!   - spec/features/rpc013-source-shape.feature
//!
//! Navigator-level layout (single owner = App):
//!   - `board::BoardView` — Kanban skeleton + view-specific footer
//!     (RPC-013); reads `&BoardStore`.
//!   - `agent::AgentView` — slim AgentView + view-specific footer
//!     (RPC-013); reads `&AgentViewStore`.
//!   - `navigator::Navigator` + `ViewMode { Board, Agent }` — top-level
//!     view that renders exactly one child per frame, full-area
//!     passthrough (RPC-013 removed the shared 1-row footer).
//!
//! These views MUST NOT import `codelet_napi`, `codelet_core`, `tarpc`,
//! or `tokio_tungstenite` directly — backend access goes through
//! `Arc<dyn FspecBackend>` only.

pub mod agent;
pub mod blocklist;
pub mod board;
pub mod navigator;
pub mod provider_settings;

pub use agent::{AgentView, RenderedChunk as AgentRenderedChunk};
pub use blocklist::{BlocklistEvent, BlocklistView};
pub use board::BoardView;
pub use navigator::{Navigator, ViewMode};
pub use provider_settings::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
