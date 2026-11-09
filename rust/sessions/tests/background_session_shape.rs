//! Feature: spec/features/move-background-session-into-codelet-sessions.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_sessions::background_session::{
    format_incoming_message, BackgroundSession, BridgeImageData, CompactionProgress,
    IncomingMessage, PromptInput, SessionError, WorkUnitContext,
};
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "background_session_shape/api.rs"]
mod api;
#[path = "background_session_shape/broadcast.rs"]
mod broadcast;
#[path = "background_session_shape/chunk_broadcast_constructor.rs"]
mod chunk_broadcast_constructor;
#[path = "background_session_shape/chunk_broadcast_emission.rs"]
mod chunk_broadcast_emission;
#[path = "background_session_shape/isolation.rs"]
mod isolation;
#[path = "background_session_shape/subprocess.rs"]
mod subprocess;

const _: fn() = api::scenario_send_input_is_rewritten_to_a_non_napi_result_type;
const _: fn() = broadcast::scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn moved_file_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("background_session.rs")
}

fn read(path: &Path) -> String {
    let mut source = std::fs::read_to_string(path).expect("source file must be readable");
    if path == moved_file_path() {
        for module in [
            "construction",
            "workspace",
            "focus",
            "metrics",
            "output",
            "transcript",
            "interactions",
            "exec_stdin",
            "completion_contract",
            "messaging",
            "history",
            "progress",
            "types",
            "agent_message",
            "compaction_handoff",
            "interruption",
        ] {
            let child = path.with_extension("").join(format!("{module}.rs"));
            source.push('\n');
            source.push_str(&std::fs::read_to_string(child).expect("session child source"));
        }
    }
    source
}

fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        match line.find("//") {
            Some(pos) => {
                out.push_str(&line[..pos]);
                out.push('\n');
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}
