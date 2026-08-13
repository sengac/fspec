//! NAPI-free agent-loop crate (RPC-072).
//!
//! This crate provides:
//!
//!   * [`FspecAgentHooks`] — a [`codelet_sessions::session_manager::SessionManagerHooks`]
//!     implementation whose `spawn_agent_loop` actually drains the per-session
//!     `input_rx` channel (the no-op default would drop it and close the
//!     channel — see RPC-072 root-cause-analysis.md).
//!   * [`agent_loop`] — the async function the hook spawns. It reads
//!     [`PromptInput`] values from `input_rx`, dispatches to the
//!     [`LlmProvider`](codelet_providers::LlmProvider) the session was
//!     created with, and emits [`StreamChunk`](codelet_rpc_types::StreamChunk)
//!     values through [`BackgroundSession::handle_output`].
//!
//! ## Scope
//!
//! RPC-072 intentionally ships the MINIMUM impl needed to flip the
//! headline acceptance criteria from BROKEN to PASSING for the
//! deterministic stub provider:
//!
//!   * Non-streaming `complete_with_tools` per prompt.
//!   * Single [`StreamChunk::Text`] + [`StreamChunk::Done`] emitted per turn.
//!   * Status flips back to [`SessionStatus::Idle`] after each turn.
//!
//! Streaming, thinking-level routing, MCP injections, lifecycle hooks,
//! tool execution, and multimodal images all remain tracked as
//! follow-up work (the NAPI side keeps its own richer copy of the agent
//! loop in `rust/napi/src/agent_loop.rs`).
//!
//! ## Crate-graph constraint
//!
//! `tests/no_napi_dependency.rs` enforces that this crate (and the
//! transitive closure reached from it) never depends on `codelet-napi`.

#![deny(unsafe_code)]

pub mod agent_loop;
pub mod agent_manager_handler;
pub mod background_output;
pub mod bridges;
pub mod deep_search_handler;
pub mod deep_search_provider_config;
pub mod dispatch;
pub mod error;
pub mod graph_search_handler;
pub mod hooks;
pub mod inject_summary_handler;
pub mod persist;
pub mod schedule_handler;
pub mod session_search_handler;
pub mod stream_chunk_json;
pub mod thinking_config;
pub mod thinking_level_detection;

pub use agent_loop::agent_loop;
pub use background_output::{BackgroundOutput, BackgroundProgressEmitter};
pub use dispatch::agent_loop_dispatch_supports_provider;
pub use error::AgentLoopError;
pub use hooks::FspecAgentHooks;

/// RPC-072: NAPI-free shim for the napi-side `is_global_chunk_callback_registered`
/// guard.
///
/// **Semantics:** "Is there a TypeScript host listening on the global
/// chunk-callback channel and capable of executing fspec commands /
/// bridge commands on our behalf?"
///
/// In the standalone fspec Rust binary the answer is **always NO** —
/// there is no NAPI TSFN, no TypeScript event loop, no
/// `sessionSendFspecResult` caller. The shim therefore returns
/// `false` so that:
///
///   * The fspec_handler closure at `agent_loop::agent_loop`
///     (`if !is_global_chunk_callback_registered() { … }`) takes the
///     in-process Rust dispatch path and routes the call through
///     [`codelet_fspec_core::dispatch_command`]. **Without this**, the
///     handler instead emits a `FspecCommandRequest` chunk and blocks
///     on `BackgroundSession::wait_for_fspec_response()` forever —
///     manifesting as a hung `Fspec(list-work-units)` tool call in
///     the TUI.
///   * The bridge command_emitter skips emission with a tracing warn,
///     which is the correct semantics for the standalone binary
///     (there is no TS to execute the relayed command).
///
/// FspecCommandRequest / FspecCommandResult chunks meant purely for
/// UI display are still emitted by the dispatch path explicitly
/// (TUI / WS bridge subscribers receive them via the per-session
/// broadcast).
#[inline]
pub fn is_global_chunk_callback_registered() -> bool {
    false
}
