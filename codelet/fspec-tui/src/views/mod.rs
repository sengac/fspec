//! View layer.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc012-board-store.feature
//!
//! RPC-012 navigator-level layout (single owner = App):
//!   - `board::BoardView` — placeholder Kanban skeleton, reads
//!     `&BoardStore`.
//!   - `agent::AgentView` — slim AgentView, reads `&AgentViewStore`.
//!   - `navigator::Navigator` + `ViewMode { Board, Agent }` — top-level
//!     view that renders exactly one child + footer per frame.
//!   - `footer::FooterView` — 1-row hint bar (unchanged from RPC-009).
//!
//! These views MUST NOT import `codelet_napi`, `codelet_core`, `tarpc`,
//! or `tokio_tungstenite` directly — backend access goes through
//! `Arc<dyn FspecBackend>` only.

pub mod agent;
pub mod board;
pub mod footer;
pub mod navigator;

pub use agent::{AgentView, RenderedChunk as AgentRenderedChunk};
pub use board::BoardView;
pub use footer::FooterView;
pub use navigator::{Navigator, ViewMode};
