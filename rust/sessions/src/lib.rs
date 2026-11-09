//! `codelet-sessions` - the NAPI-free session-manager crate.
//!
//! Hosts `SessionManager` and `BackgroundSession`, the agent loop, and the
//! tokio broadcast wiring that replaces NAPI's `GLOBAL_CHUNK_CALLBACK`.
//!
//! ## Layout
//!
//! * [`background_session`] - populated by RPC-039.
//! * [`chain_of_command`] - lifted from napi by RPC-040.
//! * [`credentials`] - lifted from napi by RPC-040.
//! * [`navigation`] - lifted from napi by RPC-040.
//! * [`session_manager`] - populated by RPC-040 with `SessionManager`,
//!   the `SessionManagerHooks` trait, and `NoopSessionManagerHooks`.

pub mod activity_focus;
mod archival_replay;
pub mod background_session;
mod blocklist_session_rules; // BUG-601
pub mod bot;
pub mod chain_of_command;
pub mod cloud_models;
pub mod codex_section;
mod committed_context;
pub mod committed_publication;
mod provider_status;
mod replay_content;
mod replay_media;
mod replay_transcript;
mod session_mcp;
mod session_restore;

pub mod conversions;
pub mod credentials;
pub mod default_model_persistence;
mod default_project;
pub mod default_thinking_level_persistence;
pub mod exec_stdin_mapping;
mod focus_reconciliation;
pub mod footer_poller;
mod handle_creation;
pub mod handle_impl;
pub mod hitl_mapping;
mod inherited_profile_sections;
mod interaction_slot;
pub mod last_used_model_persistence;
pub mod lock_state;
pub mod model_parsing;
pub mod model_resolution;
mod model_switch;
pub mod mux_config_persistence;
pub mod navigation;
mod profile_environment;
pub mod profile_limits;
mod profile_model_limits;
pub mod profile_persistence;
pub mod profile_sections;
pub mod provider_settings;
pub mod resume_isolation;
mod runtime_settings;
pub mod session_creation_helper;
pub mod session_manager;
pub mod session_tool_callbacks;
pub mod startup_model_resolution;
pub mod startup_model_utils;
pub mod subordinate_forwarding;
pub mod subordinate_output;
pub mod thinking_options;
mod tool_admission;
mod transcript_archive;
mod transcript_journal;

// Convenient re-exports so downstream crates can write
// `use codelet_sessions::SessionManagerHooks;` instead of the longer
// `use codelet_sessions::session_manager::SessionManagerHooks;`.
pub use chain_of_command::ChainOfCommand;
pub use session_manager::{NoopSessionManagerHooks, SessionManager, SessionManagerHooks};
