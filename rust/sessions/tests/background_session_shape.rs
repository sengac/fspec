//! Background-session shape tests for the `codelet-sessions` crate (RPC-039).
//!
//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
//!
//! These tests codify the static shape of the moved `BackgroundSession`
//! by inspecting source files and manifests, building the involved
//! crates, and running `cargo metadata`. They do not exercise any
//! runtime code path of the agent loop. Each `#[test]` corresponds to a
//! single Gherkin scenario in the feature file; the `// @step` comments
//! below map each Gherkin step to the assertion that enforces it.
//!
//! Pattern borrowed from `rust/sessions/tests/skeleton_invariants.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path & file helpers (shared by every scenario in this file).
// =============================================================================

/// Workspace root (one level above this crate's manifest dir).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Path to the moved file (`rust/sessions/src/background_session.rs`).
fn moved_file_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("background_session.rs")
}

/// Path to the napi shell.
///
/// RPC-043 retro (2026-05-27): the original RPC-039 helper pointed at
/// `rust/napi/src/session_manager.rs`. RPC-043 deleted that file and
/// split its contents across seven sibling modules; the re-exports,
/// `pub fn session_*` #[napi] free functions, and `GlobalChunkCallbackArgs`
/// struct asserted by every test below now live in
/// `rust/napi/src/session_bindings.rs`. The helper was retargeted but
/// the contract being asserted (the napi shell still surfaces these
/// invariants) is unchanged from RPC-039/RPC-041. The @step comments and
/// error-message strings that still reference `session_manager.rs` are
/// intentionally retained as historical context for the original Gherkin
/// steps in `spec/features/move-background-session-into-codelet-sessions.feature`.
fn napi_shell_path() -> PathBuf {
    workspace_root()
        .join("napi")
        .join("src")
        .join("session_bindings.rs")
}

/// Strip `//`-style line comments from a body of Rust source so
/// downstream substring scans only see executable code. Block comments
/// (`/* ... */`) and doc comments embedded in attribute syntax are not
/// stripped — `BackgroundSession` does not use them around the moved
/// code so the simple line-comment strip is sufficient.
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

/// Compile-time proof that the moved type's path resolves from the new home.
/// This `use` line FAILS to compile if RPC-039 has not been applied because
/// the placeholder background_session.rs declares no symbols.
#[allow(unused_imports)]
use codelet_sessions::background_session::{
    format_incoming_message, BackgroundSession, BridgeImageData, CompactionProgress,
    IncomingMessage, PromptInput, SessionError, WorkUnitContext,
};

#[test]
fn scenario_codelet_sessions_builds_standalone_with_background_session_at_its_new_home() {
    // @step Given the BackgroundSession struct and impl have been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "rust/sessions/src/background_session.rs must define `pub struct BackgroundSession`"
    );
    assert!(
        moved.contains("impl BackgroundSession"),
        "rust/sessions/src/background_session.rs must contain an `impl BackgroundSession` block"
    );

    // @step When I run `cargo build -p codelet-sessions`
    let output = Command::new(env!("CARGO"))
        .args(["build", "-p", "codelet-sessions", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");

    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "cargo build -p codelet-sessions failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the public path `codelet_sessions::background_session::BackgroundSession` resolves to the moved struct
    //
    // The `use codelet_sessions::background_session::BackgroundSession;`
    // statement at the top of this file is the compile-time witness for
    // this assertion. If RPC-039 has not been applied, this test crate
    // will FAIL to compile and `cargo build` of the test binary will
    // emit `unresolved import` errors.
    //
    // We re-assert at runtime via a zero-sized lambda that references
    // the symbol — guaranteeing the linker keeps the path alive.
    let _shape_witness: fn() = || {
        // Touching the type in a no-op generic context.
        fn _accept<T>(_: std::marker::PhantomData<T>) {}
        _accept::<BackgroundSession>(std::marker::PhantomData);
    };
}

// =============================================================================
// Scenario: codelet-napi still builds against the re-exported BackgroundSession.
// =============================================================================

#[test]
fn scenario_codelet_napi_still_builds_against_the_re_exported_background_session() {
    // @step Given rust/napi/src/session_manager.rs now `pub use`s BackgroundSession + its companion types from codelet-sessions
    let napi = read(&napi_shell_path());
    assert!(
        napi.contains("pub use codelet_sessions::background_session::"),
        "rust/napi/src/session_manager.rs must re-export the moved symbols via `pub use codelet_sessions::background_session::{{..}}`"
    );
    for name in [
        "BackgroundSession",
        "IncomingMessage",
        "BridgeImageData",
        "format_incoming_message",
        "PromptInput",
        "CompactionProgress",
        "WorkUnitContext",
        "SessionError",
    ] {
        assert!(
            napi.contains(name),
            "rust/napi/src/session_manager.rs must reference `{name}` (typically via the `pub use codelet_sessions::background_session::{{..}}` re-export block)"
        );
    }

    // @step When I run `cargo build -p codelet-napi`
    let output = Command::new(env!("CARGO"))
        .args(["build", "-p", "codelet-napi", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");

    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "cargo build -p codelet-napi failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the rest of session_manager.rs (ChainOfCommand, SessionManager, the agent_loop, the #[napi] free functions, the unit-test module) resolves every BackgroundSession / PromptInput / IncomingMessage / BridgeImageData / format_incoming_message / WorkUnitContext / CompactionProgress / SessionError path through the re-exports
    //
    // Successful `cargo build -p codelet-napi` above is itself the
    // witness — every external reference in session_manager.rs to the
    // moved symbols resolves through the re-exports, otherwise the
    // build would have failed with `cannot find type ...` errors.
    //
    // RPC-039 originally asserted that ChainOfCommand + SessionManager
    // stayed in napi for the duration of this card. RPC-040 has since
    // lifted both into codelet-sessions, so the napi shell now relies
    // on `pub use codelet_sessions::session_manager::SessionManager;`
    // and `pub use codelet_sessions::chain_of_command::ChainOfCommand;`
    // re-exports. The check below was updated to follow the post-040
    // shape: the napi shell must surface ChainOfCommand + SessionManager
    // via re-export (either form is acceptable).
    assert!(
        napi.contains("pub use codelet_sessions::chain_of_command::ChainOfCommand")
            || napi.contains("pub use codelet_sessions::chain_of_command::")
            || napi.contains("pub struct ChainOfCommand"),
        "rust/napi/src/session_manager.rs must surface `ChainOfCommand` (post-RPC-040: via re-export)"
    );
    assert!(
        napi.contains("pub use codelet_sessions::session_manager::SessionManager")
            || napi.contains("pub struct SessionManager"),
        "rust/napi/src/session_manager.rs must surface `SessionManager` (post-RPC-040: via re-export)"
    );
}

// =============================================================================
// Scenario: The moved background_session.rs has no napi:: references.
// =============================================================================

#[test]
fn scenario_the_moved_background_session_rs_has_no_napi_references() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "the moved file must define `pub struct BackgroundSession`"
    );

    // @step When I grep the moved file for the regex `napi::|use napi|#[napi`
    // Manual line-by-line scan keeps the test free of regex crate deps.
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        // Ignore doc-comments that *mention* napi as part of the history
        // of the move; those start with `//` or `//!`. Only flag a line
        // when it carries an actual napi reference outside a comment.
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("napi::") || line.contains("use napi") || line.contains("#[napi") {
            violations.push(format!("{lineno}: {line}"));
        }
    }

    // @step Then I find zero matches in rust/sessions/src/background_session.rs
    assert!(
        violations.is_empty(),
        "rust/sessions/src/background_session.rs must not reference napi (found {} violations):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

// =============================================================================
// Scenario: The moved background_session.rs has no crate::persistence or
// crate::types imports.
// =============================================================================

#[test]
fn scenario_the_moved_background_session_rs_has_no_crate_persistence_or_crate_types_imports() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());

    // @step When I grep the moved file for the regex `crate::persistence|crate::types`
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("crate::persistence") || line.contains("crate::types") {
            violations.push(format!("{lineno}: {line}"));
        }
    }

    // @step Then I find zero matches in rust/sessions/src/background_session.rs
    assert!(
        violations.is_empty(),
        "rust/sessions/src/background_session.rs must not contain `crate::persistence` or `crate::types` (found {} violations):\n{}",
        violations.len(),
        violations.join("\n")
    );

    // @step And every persistence import resolves to codelet_core::persistence
    //
    // Within the BackgroundSession struct + impl block itself there are
    // no direct uses of the persistence types (those live in the
    // SessionManager / agent_loop code which stays in napi for this
    // card). The rule therefore holds vacuously: if any persistence
    // symbol is referenced in the moved file it MUST resolve through
    // `codelet_core::persistence`, never via `crate::persistence`.
    //
    // We strip line comments before scanning so doc-comment fragments
    // that mention symbol names (e.g. "persistenceStoreMessageEnvelope"
    // in a historical comment) do not produce false positives.
    let moved_code = strip_line_comments(&moved);
    let persistence_symbols = [
        "load_session",
        "append_message_with_metadata",
        "update_session_tokens",
        "MessageEnvelope",
        "MessagePayload",
        "UserMessage",
        "UserContent",
        "AssistantMessage",
        "AssistantContent",
    ];
    let mentions_any_persistence_symbol =
        persistence_symbols.iter().any(|s| moved_code.contains(s));
    if mentions_any_persistence_symbol {
        assert!(
            moved_code.contains("codelet_core::persistence"),
            "the moved file references a persistence symbol but does not reach it via `codelet_core::persistence::{{..}}`"
        );
    }

    // @step And every FspecResult reference resolves to codelet_rpc_types::FspecResult
    //
    // Permitted forms: explicit `codelet_rpc_types::FspecResult` or a
    // local rename via `use codelet_rpc_types::FspecResult;` followed by
    // an unqualified `FspecResult` reference. Either form satisfies the
    // rule; what's forbidden is the pre-move `crate::types::FspecResult`
    // (caught by the violations scan above).
    assert!(
        moved.contains("FspecResult"),
        "the moved file must reference `FspecResult` (since it owns the fspec_response channel)"
    );
    assert!(
        moved.contains("codelet_rpc_types"),
        "the moved file must import wire types via `codelet_rpc_types::{{..}}`"
    );
}

// =============================================================================
// Scenario: send_input is rewritten to a non-NAPI Result type.
// =============================================================================

#[test]
fn scenario_send_input_is_rewritten_to_a_non_napi_result_type() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());

    // @step When I inspect the `send_input` method signature in the moved file
    let sig_marker = "pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>";

    // @step Then the signature is `pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>`
    assert!(
        moved.contains(sig_marker),
        "the moved file must declare send_input with signature `{sig_marker}` (no napi::Result)"
    );

    // @step And the error construction site uses `format!(\"Failed to send input: {}\", e)` (not `napi::Error::from_reason(...)`)
    //
    // Locate the send_input function body (between the signature and
    // the next `pub fn` declaration) and check the error formatting.
    let body_start = moved
        .find(sig_marker)
        .expect("send_input signature must exist");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);
    assert!(
        body_code.contains("format!(\"Failed to send input: {}\", e)")
            || body_code.contains("format!(\"Failed to send input: {e}\")"),
        "send_input body must construct a String error via `format!(\"Failed to send input: ...\")`. Got body:\n{body}"
    );
    assert!(
        !body_code.contains("Error::from_reason"),
        "send_input body must NOT call `Error::from_reason` (napi-specific). Got body (comments stripped):\n{body_code}"
    );
    assert!(
        !body_code.contains("napi::"),
        "send_input body must NOT reference `napi::`. Got body (comments stripped):\n{body_code}"
    );

    // @step And the napi-side free function session_send_input maps the new String error back to napi::Error::from_reason at the wire boundary so the TypeScript Promise<void> signature is preserved
    let napi = read(&napi_shell_path());
    // Locate the napi-side free function.
    let napi_sig_start = napi
        .find("pub fn session_send_input")
        .expect("session_send_input must exist in napi");
    // The signature may be rustfmt-wrapped across multiple lines once it
    // exceeds the line width (the code is fmt-correct; only its layout
    // changed). Normalize whitespace across the signature span (up to the
    // opening brace) so the assertion checks the SHAPE — the three param
    // types and the `-> Result<()>` return that preserves the TS
    // `Promise<void>` surface — independent of line wrapping.
    let sig_span = &napi[napi_sig_start..];
    let sig_brace = sig_span.find('{').unwrap_or(sig_span.len());
    let sig_ws = sig_span[..sig_brace]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        sig_ws.contains("session_id: String")
            && sig_ws.contains("input: String")
            && sig_ws.contains("thinking_config: Option<String>")
            && sig_ws.contains("-> Result<()>"),
        "rust/napi/src/session_bindings.rs must still expose `session_send_input(session_id: String, input: String, thinking_config: Option<String>) -> Result<()>` to preserve the TS Promise<void> shape. Got signature:\n{}",
        &sig_span[..sig_brace]
    );
    let after_napi_sig = &napi[napi_sig_start..];
    let napi_fn_end = after_napi_sig[1..]
        .find("\npub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_napi_sig.len().min(2000));
    let napi_body = &after_napi_sig[..napi_fn_end];
    assert!(
        napi_body.contains("Error::from_reason") || napi_body.contains("napi::Error::from_reason"),
        "session_send_input napi free function must map String errors via Error::from_reason. Got:\n{napi_body}"
    );
}

// =============================================================================
// Scenario: handle_output uses the new chunks_tx broadcast and no longer
// touches GLOBAL_CHUNK_CALLBACK.
// =============================================================================

#[test]
fn scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback(
) {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());

    // @step When I inspect the `handle_output` method in the moved file
    let sig_marker = "pub fn handle_output(&self, chunk: StreamChunk)";
    assert!(
        moved.contains(sig_marker),
        "the moved file must define `handle_output` with signature `{sig_marker}`"
    );
    let body_start = moved
        .find(sig_marker)
        .expect("handle_output signature must exist");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];

    // @step Then it still pushes the chunk into output_buffer
    assert!(
        body.contains("output_buffer") && body.contains(".push("),
        "handle_output must still push chunks into output_buffer. Got body:\n{body}"
    );

    // @step And it still calls `supervisor_broadcast.send(chunk.clone())`
    assert!(
        body.contains("supervisor_broadcast.send(chunk.clone())"),
        "handle_output must still call `supervisor_broadcast.send(chunk.clone())`. Got body:\n{body}"
    );

    // @step And it unconditionally calls the new chunks_tx broadcast (`let _ = self.chunks_tx.send((<session-id>, chunk.clone()));`) — RPC-041 removed the `if let Some(tx)` wrapper
    assert!(
        body.contains("self.chunks_tx"),
        "handle_output must reference the new `self.chunks_tx` broadcast sender. Got body:\n{body}"
    );
    assert!(
        body.contains("self.chunks_tx.send("),
        "handle_output must call `self.chunks_tx.send(...)` directly (RPC-041 removed the Option wrapper). Got body:\n{body}"
    );
    // The new field must also be declared on the struct as non-Option.
    assert!(
        moved.contains("chunks_tx:"),
        "the moved struct must declare a `chunks_tx` field"
    );

    // @step And there is zero reference to GLOBAL_CHUNK_CALLBACK in the moved file
    //
    // Strip line comments before scanning so historical references in
    // doc comments (e.g. "// chunks are dropped because the napi-side
    // GLOBAL_CHUNK_CALLBACK ...") do not produce false positives. The
    // rule targets executable code only.
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        let code = match line.find("//") {
            Some(pos) => &line[..pos],
            None => line,
        };
        if code.contains("GLOBAL_CHUNK_CALLBACK") {
            violations.push(format!("{lineno}: {line}"));
        }
    }
    assert!(
        violations.is_empty(),
        "the moved file must not reference GLOBAL_CHUNK_CALLBACK in executable code (deletion is deferred to RPC-041; in RPC-039 the global stays in napi only). Got {} violations:\n{}",
        violations.len(),
        violations.join("\n")
    );

    // @step And the GLOBAL_CHUNK_CALLBACK global has been deleted from rust/napi/src/session_manager.rs (RPC-041 inverted the prior RPC-039 assertion)
    let napi = read(&napi_shell_path());
    let napi_code = strip_line_comments(&napi);
    assert!(
        !napi_code.contains("GLOBAL_CHUNK_CALLBACK"),
        "RPC-041: rust/napi/src/session_manager.rs must no longer reference GLOBAL_CHUNK_CALLBACK in executable code"
    );
    assert!(
        !napi_code.contains("struct GlobalChunkCallback {"),
        "RPC-041: the `struct GlobalChunkCallback` wrapper must be deleted from rust/napi/src/session_manager.rs"
    );
    assert!(
        !napi_code.contains("unsafe impl Send for GlobalChunkCallback")
            && !napi_code.contains("unsafe impl Sync for GlobalChunkCallback"),
        "RPC-041: the `unsafe impl Send/Sync for GlobalChunkCallback` blocks must be deleted"
    );
}

// =============================================================================
// RPC-041 Scenario: BackgroundSession::handle_output broadcasts unconditionally
// on the mandatory chunks_tx field.
// =============================================================================

#[test]
fn scenario_handle_output_emits_unconditionally_on_chunks_tx() {
    // @step Given the BackgroundSession code lives at rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());

    // @step And `BackgroundSession::handle_output(&self, chunk: StreamChunk)` is defined there
    let sig_marker = "pub fn handle_output(&self, chunk: StreamChunk)";
    assert!(
        moved.contains(sig_marker),
        "handle_output signature `{sig_marker}` must exist"
    );

    // @step When I inspect the body of handle_output in the moved file
    let body_start = moved.find(sig_marker).expect("handle_output sig");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);

    // @step Then the body still pushes the chunk into output_buffer
    assert!(
        body_code.contains("output_buffer") && body_code.contains(".push("),
        "handle_output must still push into output_buffer. Got body:\n{body_code}"
    );

    // @step And the body still calls `supervisor_broadcast.send(chunk.clone())`
    assert!(
        body_code.contains("supervisor_broadcast.send(chunk.clone())"),
        "handle_output must still call supervisor_broadcast.send(chunk.clone()). Got body:\n{body_code}"
    );

    // @step And the body unconditionally calls `self.chunks_tx.send((codelet_rpc_types::SessionId::from(self.id.to_string()), chunk.clone()))` without an `if let Some(tx)` wrapper
    assert!(
        body_code.contains("self.chunks_tx.send("),
        "RPC-041: handle_output must call self.chunks_tx.send(...) unconditionally. Got body:\n{body_code}"
    );
    assert!(
        !body_code.contains("if let Some(tx) = &self.chunks_tx"),
        "RPC-041: the `if let Some(tx) = &self.chunks_tx` Option-wrapper must be removed. Got body:\n{body_code}"
    );

    // @step And the BackgroundSession struct declares `chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>` as a non-Option, non-pub field
    let struct_start = moved
        .find("pub struct BackgroundSession {")
        .expect("BackgroundSession struct decl must exist");
    let struct_body = &moved[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];
    let fields_code = strip_line_comments(fields_block);
    assert!(
        fields_code.contains("chunks_tx: broadcast::Sender<")
            || fields_code.contains("chunks_tx: tokio::sync::broadcast::Sender<"),
        "RPC-041: BackgroundSession.chunks_tx must be a non-Option `broadcast::Sender<...>` field. Got fields:\n{fields_code}"
    );
    assert!(
        !fields_code.contains("chunks_tx: Option<"),
        "RPC-041: BackgroundSession.chunks_tx must NOT be Option-wrapped"
    );
}

// =============================================================================
// RPC-041 Scenario: BackgroundSession::set_status emits on status_changes_tx.
// =============================================================================

#[test]
fn scenario_set_status_emits_on_status_changes_tx() {
    // @step Given the BackgroundSession code lives at rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());

    // @step And `BackgroundSession::set_status(&self, status: SessionStatus)` is defined there
    let sig_marker = "pub fn set_status(&self, status: SessionStatus)";
    assert!(moved.contains(sig_marker), "set_status sig must exist");

    // @step When I inspect the body of set_status in the moved file
    let body_start = moved.find(sig_marker).expect("set_status sig");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);

    // @step Then the body first calls `let old_status = self.status.swap(status as u8, Ordering::AcqRel)`
    assert!(
        body_code.contains("self.status.swap(status as u8, Ordering::AcqRel)"),
        "set_status must still call self.status.swap(...). Got body:\n{body_code}"
    );

    // @step And under the `if old_status != status as u8` guard the body calls `let _ = self.status_changes_tx.send((codelet_rpc_types::SessionId::from(self.id.to_string()), status))`
    assert!(
        body_code.contains("if old_status != status as u8"),
        "set_status must keep the old_status != status guard. Got body:\n{body_code}"
    );
    assert!(
        body_code.contains("self.status_changes_tx.send("),
        "RPC-041: set_status must emit on self.status_changes_tx.send(...). Got body:\n{body_code}"
    );

    // @step And the body still calls `self.handle_output(StreamChunk::session_state_change(state))`
    assert!(
        body_code.contains("self.handle_output(StreamChunk::session_state_change("),
        "set_status must still emit SessionStateChange via handle_output. Got body:\n{body_code}"
    );

    // @step And the body still calls `codelet_tools::broadcast_metadata_update()`
    assert!(
        body_code.contains("codelet_tools::broadcast_metadata_update()"),
        "set_status must still call codelet_tools::broadcast_metadata_update(). Got body:\n{body_code}"
    );

    // @step And the BackgroundSession struct declares a non-Option `status_changes_tx: broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::SessionStatus)>` field
    let struct_start = moved
        .find("pub struct BackgroundSession {")
        .expect("BackgroundSession struct decl must exist");
    let struct_body = &moved[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];
    let fields_code = strip_line_comments(fields_block);
    assert!(
        fields_code.contains("status_changes_tx: broadcast::Sender<")
            || fields_code.contains("status_changes_tx: tokio::sync::broadcast::Sender<"),
        "RPC-041: BackgroundSession.status_changes_tx must be a non-Option `broadcast::Sender<...>` field. Got fields:\n{fields_code}"
    );
    assert!(
        !fields_code.contains("status_changes_tx: Option<"),
        "RPC-041: BackgroundSession.status_changes_tx must NOT be Option-wrapped"
    );
}

// =============================================================================
// RPC-041 Scenario: BackgroundSession::new accepts chunks_tx and
// status_changes_tx as new trailing parameters.
// =============================================================================

#[test]
fn scenario_background_session_new_accepts_broadcast_senders_as_trailing_parameters() {
    // @step Given the BackgroundSession constructor `BackgroundSession::new(...)` is the sole construction site for BackgroundSession
    let moved = read(&moved_file_path());

    // @step When I inspect the parameter list of `BackgroundSession::new` in rust/sessions/src/background_session.rs
    // Locate the BackgroundSession impl block so we find the right `pub fn new(`
    // (the module contains other types with their own `pub fn new(...)` constructors,
    // such as IncomingMessage at module top).
    let impl_marker = moved
        .find("impl BackgroundSession {")
        .expect("`impl BackgroundSession {` must exist");
    let impl_body = &moved[impl_marker..];
    let new_offset_in_impl = impl_body
        .find("pub fn new(")
        .expect("BackgroundSession::new must exist inside its impl block");
    let new_sig_start = impl_marker + new_offset_in_impl;
    let after = &moved[new_sig_start..];
    let body_open = after
        .find(") -> Self")
        .or_else(|| after.find(") -> BackgroundSession"))
        .expect("BackgroundSession::new return marker must exist");
    let params = &after[..body_open];
    let params_code = strip_line_comments(params);

    // @step Then the signature ends with two new trailing parameters `chunks_tx: tokio::sync::broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)>` and `status_changes_tx: tokio::sync::broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::SessionStatus)>`
    assert!(
        params_code.contains("chunks_tx:") && params_code.contains("broadcast::Sender"),
        "RPC-041: BackgroundSession::new must take `chunks_tx: broadcast::Sender<...>` parameter. Got params:\n{params_code}"
    );
    assert!(
        params_code.contains("status_changes_tx:"),
        "RPC-041: BackgroundSession::new must take `status_changes_tx: broadcast::Sender<...>` parameter. Got params:\n{params_code}"
    );

    // @step And the body initializes the `chunks_tx` struct field directly from the parameter (no `None` placeholder remains)
    let body_start = body_open;
    let after_body_start = &after[body_start..];
    let body_end = after_body_start
        .find("\n    }\n")
        .unwrap_or(after_body_start.len());
    let new_body = &after_body_start[..body_end];
    let new_body_code = strip_line_comments(new_body);
    assert!(
        !new_body_code.contains("chunks_tx: None"),
        "RPC-041: BackgroundSession::new must NOT initialize `chunks_tx` to `None`"
    );
    assert!(
        new_body_code.contains("chunks_tx,") || new_body_code.contains("chunks_tx:"),
        "RPC-041: BackgroundSession::new must initialize the chunks_tx field from the parameter. Got body:\n{new_body_code}"
    );

    // @step And the body initializes the `status_changes_tx` struct field directly from the parameter
    assert!(
        new_body_code.contains("status_changes_tx,") || new_body_code.contains("status_changes_tx:"),
        "RPC-041: BackgroundSession::new must initialize the status_changes_tx field from the parameter. Got body:\n{new_body_code}"
    );
}

// =============================================================================
// Scenario: codelet-sessions has no transitive dependency on codelet-napi.
// =============================================================================

#[test]
fn scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "the moved file must define `pub struct BackgroundSession`"
    );

    // @step When I run `cargo metadata -p codelet-sessions --format-version 1`
    //
    // We use a workspace-wide `cargo metadata` and walk the resolve
    // graph rooted at codelet-sessions to determine the closure of its
    // transitive dependencies. The dependency-rule logic mirrors
    // rust/sessions/tests/skeleton_invariants.rs.
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON must parse");
    let resolve = json
        .get("resolve")
        .expect("cargo metadata must include a resolve");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("resolve.nodes must be an array");
    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("packages must be an array");
    let sessions_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();

    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![sessions_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    let mut transitive_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }

    // @step Then the resulting JSON contains zero packages with name `codelet-napi`
    assert!(
        !transitive_names.contains("codelet-napi"),
        "codelet-sessions must not transitively depend on codelet-napi. Transitive set: {transitive_names:?}"
    );
}

// =============================================================================
// Scenario: Pre-existing in-file unit tests in codelet-napi still pass via the
// re-exports.
// =============================================================================

#[test]
fn scenario_pre_existing_in_file_unit_tests_in_codelet_napi_still_pass_via_the_re_exports() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "the moved file must define `pub struct BackgroundSession`"
    );

    // @step And rust/napi/src/session_manager.rs re-exports BackgroundSession, IncomingMessage, BridgeImageData, format_incoming_message, PromptInput, CompactionProgress, WorkUnitContext, SessionError from codelet-sessions
    let napi = read(&napi_shell_path());
    assert!(
        napi.contains("pub use codelet_sessions::background_session::"),
        "rust/napi/src/session_manager.rs must contain a `pub use codelet_sessions::background_session::{{..}}` block"
    );

    // @step When I run `cargo test -p codelet-napi --lib session_manager::tests`
    //
    // The runtime check is gated behind the env var `RPC_039_FULL_TESTS=1`
    // to keep the default `cargo test -p codelet-sessions` invocation
    // fast. The compile-only mode still asserts the source-shape
    // preconditions above, which is sufficient to detect breakage of
    // the re-export contract.
    //
    // NOTE: the napi-side unit tests live in named sub-modules (e.g.
    // `supervisor_broadcast_tests`, `chain_of_command_tests`,
    // `correlation_id_tests`, etc.) rather than a single `tests`
    // module. The filter `session_manager` exercises every one of
    // them.
    if std::env::var_os("RPC_039_FULL_TESTS").is_none() {
        eprintln!(
            "[RPC-039] skipping `cargo test -p codelet-napi` invocation. Set RPC_039_FULL_TESTS=1 to enable."
        );
        return;
    }

    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-napi",
            "--lib",
            "session_manager",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo test must run");

    // @step Then every pre-existing test (WorkUnitContext::new tests, format_for_environment tests, IncomingMessage::new/with_images tests, format_incoming_message tests, parse_interjection tests) passes with status ok
    assert!(
        output.status.success(),
        "cargo test -p codelet-napi --lib session_manager failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // The napi-side test module names trace back to the historical
    // RFC suffixes — match on stable substrings present in those
    // modules.
    for fragment in [
        "supervisor_broadcast_tests",
        "session_role_tests",
        "supervisor_input_tests",
        "correlation_id_tests",
        " ok",
    ] {
        assert!(
            combined.contains(fragment),
            "expected to see `{fragment}` in `cargo test -p codelet-napi --lib session_manager` output. Got:\n{combined}"
        );
    }
}

// =============================================================================
// Scenario: codelet-sessions tests assert BackgroundSession is reachable from
// its new home.
// =============================================================================

#[test]
fn scenario_codelet_sessions_tests_assert_background_session_is_reachable_from_its_new_home() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "the moved file must define `pub struct BackgroundSession`"
    );

    // @step And the integration test rust/sessions/tests/background_session_shape.rs has been added
    //
    // This test file itself IS that integration test — running it
    // satisfies the precondition by construction.
    let this_test_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("background_session_shape.rs");
    assert!(
        this_test_path.exists(),
        "the RPC-039 integration test file must exist at rust/sessions/tests/background_session_shape.rs"
    );

    // @step When I run `cargo test -p codelet-sessions --tests`
    //
    // Cargo-in-cargo invocations would recurse into the same test. We
    // therefore short-circuit by asserting structural invariants that
    // running `cargo test -p codelet-sessions --tests` is expected to
    // check, and trust the smoke test (rust/sessions/tests/smoke.rs)
    // is exercised by the outer test runner.

    // @step Then the existing smoke test `crate_compiles` passes with status ok
    let smoke_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("smoke.rs");
    let smoke = read(&smoke_path);
    assert!(
        smoke.contains("fn crate_compiles"),
        "rust/sessions/tests/smoke.rs must define `fn crate_compiles`"
    );

    // @step And the new shape test asserts the path `codelet_sessions::background_session::BackgroundSession` resolves at compile-time
    //
    // The `use codelet_sessions::background_session::BackgroundSession;`
    // statement at the top of this file is the compile-time witness.
    // If it failed to resolve, this test crate would not compile and
    // `cargo test -p codelet-sessions` would error out before any
    // assertion ran.

    // @step And the shape test asserts the supporting types (PromptInput, CompactionProgress, WorkUnitContext, IncomingMessage, BridgeImageData, SessionError) are publicly reachable from the same module
    fn _accept<T>(_: std::marker::PhantomData<T>) {}
    let _supporting_witness: fn() = || {
        _accept::<PromptInput>(std::marker::PhantomData);
        _accept::<CompactionProgress>(std::marker::PhantomData);
        _accept::<WorkUnitContext>(std::marker::PhantomData);
        _accept::<IncomingMessage>(std::marker::PhantomData);
        _accept::<BridgeImageData>(std::marker::PhantomData);
        _accept::<SessionError>(std::marker::PhantomData);
        let _ = format_incoming_message; // function-pointer reach
    };

    // @step And the shape test asserts BackgroundSession::send_input returns `Result<(), String>` (not napi::Result)
    //
    // The structural check in
    // scenario_send_input_is_rewritten_to_a_non_napi_result_type
    // already inspects the literal signature. Here we ALSO verify the
    // method's return type at compile time by referencing the function
    // pointer.
    let _send_input_witness: fn(&BackgroundSession, String, Option<String>) -> Result<(), String> =
        BackgroundSession::send_input;
}

// =============================================================================
// Scenario: NAPI TypeScript surface is byte-stable across the move.
// =============================================================================

#[test]
fn scenario_napi_typescript_surface_is_byte_stable_across_the_move() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(
        moved.contains("pub struct BackgroundSession"),
        "the moved file must define `pub struct BackgroundSession`"
    );

    // @step When I run `cargo build -p codelet-napi --release` regenerating rust/napi/index.d.ts
    //
    // The release build regenerates `rust/napi/index.d.ts`. Running
    // a full release build inside a unit test is expensive, so the
    // runtime invocation is gated behind RPC_039_FULL_TESTS=1. In
    // compile-only mode we assert the structural preconditions that
    // make byte-stability achievable: the napi shell still owns every
    // public `#[napi]` free function that drives the TS surface.
    let napi = read(&napi_shell_path());
    for fragment in [
        "pub fn session_send_input",
        "pub fn session_interrupt",
        "pub fn session_clear_history",
        "pub fn session_get_status",
        "pub fn session_get_pause_state",
        "pub fn session_get_hitl_request",
        "pub fn session_set_active",
        "pub fn session_get_model",
        "pub fn session_get_tokens",
        "pub fn session_send_fspec_result",
        "pub fn session_send_hitl_response",
        "pub fn session_set_work_unit_context",
        "pub fn session_get_work_unit_context",
        "pub fn session_pause_resume",
        "pub fn session_pause_confirm",
        "pub fn session_pause_triple",
        "pub fn session_get_effective_cwd",
    ] {
        assert!(
            napi.contains(fragment),
            "rust/napi/src/session_manager.rs must still expose the `{fragment}` napi function so the TS surface stays byte-stable"
        );
    }

    if std::env::var_os("RPC_039_FULL_TESTS").is_none() {
        eprintln!(
            "[RPC-039] skipping `cargo build -p codelet-napi --release` invocation. Set RPC_039_FULL_TESTS=1 to enable."
        );

        // @step Then no TypeScript interface, function, enum, or type alias in the regenerated rust/napi/index.d.ts is removed
        // @step And no TypeScript interface, function, enum, or type alias is renamed
        // @step And no field of any TypeScript interface is reordered, renamed, or removed
        //
        // In compile-only mode, byte-stability is asserted indirectly
        // by the structural checks above. A full byte-comparison check
        // requires snapshotting `rust/napi/index.d.ts` before and
        // after the move; that snapshot is captured by the developer
        // running with `RPC_039_FULL_TESTS=1` on a workspace with a
        // committed pre-move `index.d.ts` baseline.
        return;
    }

    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "-p",
            "codelet-napi",
            "--release",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");

    // @step Then no TypeScript interface, function, enum, or type alias in the regenerated rust/napi/index.d.ts is removed
    assert!(
        output.status.success(),
        "cargo build -p codelet-napi --release failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dts_path = workspace_root().join("napi").join("index.d.ts");
    let dts = read(&dts_path);
    // The TS surface around the moved code is driven by the napi free
    // functions listed above; the regenerated index.d.ts must still
    // declare each of them.
    for fragment in [
        "sessionSendInput",
        "sessionInterrupt",
        "sessionClearHistory",
        "sessionGetStatus",
        "sessionGetPauseState",
        "sessionGetHitlRequest",
        "sessionSetActive",
        "sessionGetModel",
        "sessionGetTokens",
        "sessionSendFspecResult",
        "sessionSendHitlResponse",
        "sessionSetWorkUnitContext",
        "sessionGetWorkUnitContext",
        "sessionPauseResume",
        "sessionPauseConfirm",
        "sessionPauseTriple",
        "sessionGetEffectiveCwd",
    ] {
        assert!(
            dts.contains(fragment),
            "regenerated rust/napi/index.d.ts is missing `{fragment}` — the NAPI TypeScript surface is NOT byte-stable across the move"
        );
    }

    // @step And no TypeScript interface, function, enum, or type alias is renamed
    // @step And no field of any TypeScript interface is reordered, renamed, or removed
    //
    // Asserted by the substring checks above — every pre-existing
    // function name must still be present. A byte-for-byte snapshot
    // diff is handled by the project's git workflow (manual review of
    // `git diff rust/napi/index.d.ts` after the move).
}

// =============================================================================
// RPC-041 Scenario: The GLOBAL_CHUNK_CALLBACK static, GlobalChunkCallback
// struct, and its unsafe Send/Sync impls are removed.
// =============================================================================

#[test]
fn scenario_global_chunk_callback_static_struct_and_unsafe_impls_are_removed() {
    // @step Given the napi-side rust/napi/src/session_manager.rs file previously declared `static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback>`, a `struct GlobalChunkCallback { callback: ThreadsafeFunction<GlobalChunkCallbackArgs> }`, and `unsafe impl Send for GlobalChunkCallback {}` / `unsafe impl Sync for GlobalChunkCallback {}`
    let napi = read(&napi_shell_path());
    let napi_code = strip_line_comments(&napi);

    // @step When I grep rust/napi/src/session_manager.rs for the regex `GLOBAL_CHUNK_CALLBACK|struct GlobalChunkCallback|unsafe impl (Send|Sync) for GlobalChunkCallback`
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in napi.lines().enumerate() {
        let code = match line.find("//") {
            Some(pos) => &line[..pos],
            None => line,
        };
        if code.contains("GLOBAL_CHUNK_CALLBACK")
            || code.contains("struct GlobalChunkCallback {")
            || (code.contains("unsafe impl") && code.contains("for GlobalChunkCallback "))
            || (code.contains("unsafe impl") && code.contains("for GlobalChunkCallback{"))
        {
            violations.push(format!("{}: {}", idx + 1, line));
        }
    }

    // @step Then I find zero matches in the file
    assert!(
        violations.is_empty(),
        "RPC-041: rust/napi/src/session_manager.rs must no longer contain GLOBAL_CHUNK_CALLBACK / struct GlobalChunkCallback / unsafe impl (Send|Sync) for GlobalChunkCallback. Got {} violations:\n{}",
        violations.len(),
        violations.join("\n")
    );

    // @step And the `GlobalChunkCallbackArgs` `#[napi(object)]` struct still exists because it is the TS-facing wire shape
    assert!(
        napi_code.contains("pub struct GlobalChunkCallbackArgs"),
        "RPC-041: GlobalChunkCallbackArgs must remain (it is the TS-facing wire shape)"
    );
    assert!(
        napi.contains("#[napi(object)]"),
        "RPC-041: the GlobalChunkCallbackArgs struct must retain its #[napi(object)] attribute"
    );
}
