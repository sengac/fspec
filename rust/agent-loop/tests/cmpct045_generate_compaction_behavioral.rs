#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock
)]
//! Feature: spec/features/generate-compaction-tool.feature
//!
//! CMPCT-045: behavioral coverage for the GenerateCompaction tool's
//! agent-loop handler side — ephemeral compactor sub-agent dispatch,
//! handler-side pinning (calling-session stash vs cross-session
//! immediate pin), fallback DAGs, lifecycle events, and the
//! provider-builder tool wiring.
//!
//! The handler closure is registered through the real
//! `register_generate_compaction_handler` (the same entry point the
//! agent loop uses at session creation) and driven through the real
//! tools-crate `GenerateCompactionTool::call` — the only fakes are a
//! wiremock LLM endpoint (openai provider via OPENAI_BASE_URL) and
//! process-global env seams. All tests serialize on a process-global
//! guard (env vars + data dir + tool-handler registries are
//! process-global) mirroring the `serial_test` precedent.

#[path = "cmpct056/selected.rs"]
mod selected;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use codelet_core::compaction::{parse_dag_nodes, wrap_dag_content, ConversationTurn};
use codelet_rpc_types::{SessionState, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::session_manager::SessionManager;
use codelet_tools::{
    clear_all_generate_compaction_handlers, has_generate_compaction_handler,
    set_generate_compaction_handler, GenerateCompactionArgs, GenerateCompactionHandler,
    GenerateCompactionTool, ToolError,
};
use rig::message::{Message, UserContent};
use rig::tool::Tool;
use rig::OneOrMany;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use serial_test::serial;

include!("cmpct045/part_01.rs");
include!("cmpct045/part_02.rs");
include!("cmpct045/part_03.rs");
include!("cmpct045/part_04.rs");
include!("cmpct045/part_05.rs");
include!("cmpct045/part_06.rs");
include!("cmpct045/part_07.rs");
