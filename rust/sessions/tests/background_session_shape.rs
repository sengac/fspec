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
#[path = "background_session_shape/isolation.rs"]
mod isolation;
#[path = "background_session_shape/subprocess.rs"]
mod subprocess;
#[path = "background_session_shape/typescript.rs"]
mod typescript;

const _: fn() =
    subprocess::scenario_codelet_napi_still_builds_against_the_re_exported_background_session;
const _: fn() = api::scenario_send_input_is_rewritten_to_a_non_napi_result_type;
const _: fn() = broadcast::scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback;
const _: fn() = subprocess::scenario_pre_existing_in_file_unit_tests_in_codelet_napi_still_pass_via_the_re_exports;
const _: fn() = typescript::scenario_napi_typescript_surface_is_byte_stable_across_the_move;
const _: fn() =
    isolation::scenario_global_chunk_callback_static_struct_and_unsafe_impls_are_removed;

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

fn napi_shell_path() -> PathBuf {
    workspace_root()
        .join("napi")
        .join("src")
        .join("session_bindings.rs")
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
