//! codelet-fspec-tui: ratatui app shell + transport-agnostic FspecBackend
//! trait + embedded/WebSocket implementations.
//!
//! Feature: spec/features/fspec-tui-app-shell.feature
//! Card: RPC-008 (parent RPC-002, depends on RPC-007).
//!
//! This crate hosts the RPC-002 Slice 01 foundation (Compositor + Component
//! + EventResult + App) plus the transport-plumbing parts of Slice 02.
//!
//! Renders only a placeholder Hello component and an ESC-dismissible
//! HelpDialog. Real list view + REPL arrives in RPC-009; binary entry
//! points arrive in RPC-010.
//!
//! ## Public surface
//!
//! - [`FspecBackend`] — the transport-agnostic trait every consumer
//!   in RPC-009/RPC-010 holds via `Arc<dyn FspecBackend>`.
//! - [`EmbeddedFspecBackend`] — wraps `codelet_rpc_embedded::EmbeddedTransport`,
//!   preserves the RPC-005 Q9 host-supplied-Handle invariant at the trait
//!   boundary (constructor takes a non-defaulted `tokio::runtime::Handle`).
//! - [`WebSocketFspecBackend`] — wraps `codelet_rpc_server::FspecWsClient`,
//!   opens the connection via `tokio_tungstenite::connect_async`.
//! - [`Compositor`] / [`Component`] / [`Priority`] / [`EventResult`] —
//!   layered priority dispatcher + the trait every UI element implements.
//! - [`App`] — application root + run loop (rule [10] [11]).
//! - [`Theme`] — shared color palette read by every layer (rule [10] [16]).
//! - [`TerminalGuard`] — RAII alt-screen + raw mode + mouse-capture +
//!   bracketed-paste guard with idempotent panic hook (rule [12]).

pub mod app;
pub mod components;
pub mod compositor;
pub mod mouse;
pub mod store;
pub mod terminal;
pub(crate) mod text_normalize;
pub mod theme;
pub mod transport;
pub mod views;

#[cfg(test)]
mod compositor_tests;

pub use app::{parse_slash_command, synth_key, App, SlashCommandParse};
pub use components::attachment_picker_dialog::{
    AttachmentPickerDialog, ATTACHMENT_PICKER_DIALOG_ID,
};
pub use components::create_session_dialog::{
    CreateSessionDialog, CreateSessionOption, CREATE_SESSION_DIALOG_ID,
};
pub use components::dialog_theme::Accent;
pub use components::hello::HelloComponent;
pub use components::help_dialog::HelpDialog;
pub use components::role_dialog::{RoleDialog, ROLE_DIALOG_ID};
pub use components::thinking_level_dialog::{ThinkingLevelDialog, THINKING_LEVEL_DIALOG_ID};
pub use components::{Action, Callback, Component, EventResult, Priority};
pub use compositor::Compositor;
pub use store::agent_view::extract_tool_args_display;
pub use store::{AgentViewStore, BoardStore, IsolationState, SessionContext, COLUMN_ORDER};
pub use terminal::TerminalGuard;
pub use theme::Theme;
pub use transport::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
pub use views::agent::rendered_chunk::ChunkKind;
pub use views::agent::{
    ConfirmDialog, ConfirmDialogOutcome, RenderedChunk, ResumeSessionView,
    ResumeSessionViewOutcome, SearchHistoryView, SearchHistoryViewOutcome, SLASH_COMMANDS,
};
pub use views::{AgentView, BoardView, Navigator, ViewMode};
