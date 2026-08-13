// Feature: spec/features/global-chunk-callback-napi.feature
//
// Tests for global chunk callback NAPI architecture.
// Rust exposes a single global callback via NAPI that TypeScript registers once at startup.
// ALL chunks from ALL sessions go through this ONE callback with signature (session_id, chunk).
// Rust has ZERO knowledge of which session is active/attached - it's a pure emitter.
// This replaces the per-session attach()/detach() pattern completely.

use std::fs;
use std::path::Path;

/// Helper to read the Rust source file
fn read_session_manager_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/session_manager.rs");
    fs::read_to_string(&path).expect("Failed to read session_manager.rs")
}

/// Helper to read the NAPI TypeScript declaration file
fn read_napi_declarations() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("index.d.ts");
    fs::read_to_string(&path).expect("Failed to read index.d.ts")
}

mod register_global_chunk_callback_at_startup {
    use super::*;

    /// Feature: Global chunk callback NAPI for session-agnostic chunk emission
    /// Scenario: Register global chunk callback at startup
    ///
    /// @step Given no global chunk callback is registered
    /// @step When TypeScript calls sessionSetGlobalChunkCallback with a callback function
    /// @step Then Rust should store the callback in a global static
    /// @step And subsequent chunk emissions should use this callback
    #[test]
    fn test_global_callback_registration_exports() {
        let declarations = read_napi_declarations();

        // Verify sessionSetGlobalChunkCallback is exported in NAPI declarations
        // The declaration may span multiple lines, so check for the function name
        let has_global_callback_export = declarations.contains("sessionSetGlobalChunkCallback(")
            && declarations.contains("callback:");

        assert!(
            has_global_callback_export,
            "BRIDGE-012: NAPI should export sessionSetGlobalChunkCallback for TypeScript to call at startup."
        );
    }

    #[test]
    fn test_global_callback_accepts_callback_parameter() {
        let declarations = read_napi_declarations();

        // Verify the callback parameter accepts a function
        // The declaration may span multiple lines with callback: on the next line
        let has_callback_param = declarations.contains("sessionSetGlobalChunkCallback(")
            && declarations.contains("callback: (");

        assert!(
            has_callback_param,
            "BRIDGE-012: sessionSetGlobalChunkCallback should accept a callback function parameter."
        );
    }

    #[test]
    fn test_global_callback_returns_void() {
        let source = read_session_manager_source();

        // Verify sessionSetGlobalChunkCallback returns Result<()> in Rust
        let has_correct_signature = source
            .contains("pub fn session_set_global_chunk_callback(callback:")
            && source.contains("-> Result<()>");

        assert!(
            has_correct_signature,
            "BRIDGE-012: session_set_global_chunk_callback should return Result<()>."
        );
    }
}

mod emit_chunk_with_session_id_through_global_callback {
    use super::*;

    /// Feature: spec/features/replace-global-chunk-callback-with-broadcast.feature
    /// Scenario: Emit chunk with session_id through global callback (RPC-041 inverted)
    ///
    /// @step Given the chunk fan-out has been replaced by a tokio::broadcast subscriber
    /// @step When the session emits a TextDelta chunk via handle_output in codelet-sessions
    /// @step Then the napi shell must no longer reference GLOBAL_CHUNK_CALLBACK in executable code
    /// @step And the napi shell must subscribe to SessionManager::instance().chunks_tx() instead
    #[test]
    fn test_handle_output_does_not_use_global_callback() {
        let source = read_session_manager_source();

        // RPC-041: handle_output now lives in codelet-sessions and routes through
        // self.chunks_tx.send(...). The napi shell must NOT mention GLOBAL_CHUNK_CALLBACK.
        let mut violations: Vec<String> = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            let code = match line.find("//") {
                Some(pos) => &line[..pos],
                None => line,
            };
            if code.contains("GLOBAL_CHUNK_CALLBACK") {
                violations.push(format!("{}: {}", idx + 1, line));
            }
        }
        assert!(
            violations.is_empty(),
            "RPC-041: rust/napi/src/session_manager.rs must no longer reference GLOBAL_CHUNK_CALLBACK in executable code. Got {} violations:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    #[test]
    fn test_global_callback_receives_session_id_via_broadcast() {
        let source = read_session_manager_source();

        // RPC-041: the subscriber task in session_set_global_chunk_callback receives the
        // (SessionId, StreamChunk) tuple from `SessionManager::instance().chunks_tx().subscribe()`
        // and forwards `session_id` (sid.to_string()) into the JS ThreadsafeFunction.
        assert!(
            source.contains(".chunks_tx().subscribe()"),
            "RPC-041: rust/napi/src/session_manager.rs must subscribe to chunks_tx via SessionManager::instance().chunks_tx().subscribe()"
        );
    }

    #[test]
    fn test_global_callback_receives_chunk_via_broadcast() {
        let source = read_session_manager_source();

        // RPC-041: the subscriber forwards each (SessionId, StreamChunk) tuple to the JS
        // ThreadsafeFunction. The new CHUNK_FANOUT_TSFN static is the storage location.
        assert!(
            source.contains("CHUNK_FANOUT_TSFN"),
            "RPC-041: rust/napi/src/session_manager.rs must declare CHUNK_FANOUT_TSFN as the new TSFN storage"
        );
    }
}

mod multiple_sessions_emit_through_same_global_callback {
    use super::*;

    /// Feature: Global chunk callback NAPI for session-agnostic chunk emission
    /// Scenario: Multiple sessions emit through same global callback
    ///
    /// @step Given a global chunk callback is registered
    /// @step And session "session-a" exists
    /// @step And session "session-b" exists
    /// @step When session "session-a" emits a chunk
    /// @step And session "session-b" emits a chunk
    /// @step Then GLOBAL_CHUNK_CALLBACK no longer exists as a static (RPC-041 replaced it with a tokio::broadcast subscriber)
    #[test]
    fn test_global_callback_static_is_removed() {
        let source = read_session_manager_source();

        // RPC-041: the OnceCell<GlobalChunkCallback> static is deleted; the per-session fan-out
        // is owned by SessionManager::chunks_tx() and a CHUNK_FANOUT_TSFN OnceCell stores the
        // ThreadsafeFunction the napi subscriber forwards into.
        let has_static_global_callback = source.contains("static GLOBAL_CHUNK_CALLBACK");
        assert!(
            !has_static_global_callback,
            "RPC-041: GLOBAL_CHUNK_CALLBACK static must be deleted from rust/napi/src/session_manager.rs"
        );
        assert!(
            source.contains("CHUNK_FANOUT_TSFN"),
            "RPC-041: the replacement CHUNK_FANOUT_TSFN static must exist"
        );
    }

    /// @step And each session's chunk emit still carries its self.id (through the tuple delivered on chunks_tx)
    #[test]
    fn test_each_session_id_still_flows_through_broadcast() {
        // RPC-041: the BackgroundSession now lives in codelet-sessions. The id flows through
        // chunks_tx as `(SessionId::from(self.id.to_string()), chunk)`. This test asserts the
        // chunks_tx-based contract in the codelet-sessions crate via grep of the moved file.
        use std::path::Path;
        let bg_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("napi manifest_dir parent")
            .join("sessions")
            .join("src")
            .join("background_session.rs");
        let bg = std::fs::read_to_string(&bg_path)
            .expect("rust/sessions/src/background_session.rs must exist");
        assert!(
            bg.contains("self.chunks_tx.send("),
            "RPC-041: BackgroundSession::handle_output must call self.chunks_tx.send(...)"
        );
        assert!(
            bg.contains("self.id.to_string()"),
            "RPC-041: BackgroundSession::handle_output must pass self.id.to_string() into the chunks_tx tuple"
        );
    }
}

mod no_attachment_state_in_rust {
    use super::*;

    /// Feature: Global chunk callback NAPI for session-agnostic chunk emission
    /// Scenario: No attachment state in Rust
    ///
    /// @step Given a session exists
    /// @step When I inspect the BackgroundSession struct
    /// @step Then there should be no is_attached field
    #[test]
    fn test_no_is_attached_field_in_background_session() {
        let source = read_session_manager_source();

        // Search for is_attached field declaration in BackgroundSession struct
        // Must be an actual field declaration with a colon, not a comment about it
        // The pattern "is_attached: AtomicBool" at start of line (after whitespace)
        // indicates an actual field, not a comment like "// - is_attached: AtomicBool"
        let has_is_attached_field = source.lines().any(|line| {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("///") {
                return false;
            }
            // Check for field declaration
            trimmed.starts_with("is_attached: AtomicBool")
                || trimmed.starts_with("is_attached: bool")
        });

        assert!(
            !has_is_attached_field,
            "BRIDGE-012: BackgroundSession should NOT have is_attached field. \
             Remove is_attached: AtomicBool from the struct."
        );
    }

    /// @step And there should be no attached_callback field
    #[test]
    fn test_no_attached_callback_field_in_background_session() {
        let source = read_session_manager_source();

        // Search for attached_callback field declaration (not in comments)
        let has_attached_callback_field = source.lines().any(|line| {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("///") {
                return false;
            }
            // Check for field declaration
            trimmed.starts_with("attached_callback:")
        });

        assert!(
            !has_attached_callback_field,
            "BRIDGE-012: BackgroundSession should NOT have attached_callback field. \
             Remove attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>> from the struct."
        );
    }

    /// @step And there should be no attach method
    #[test]
    fn test_no_attach_method_in_background_session() {
        let source = read_session_manager_source();

        // Search for pub fn attach method (not in comments)
        // We're looking for the method on BackgroundSession, not the NAPI export
        let has_attach_method = source.lines().any(|line| {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("///") {
                return false;
            }
            // Check for method declaration
            trimmed.starts_with("pub fn attach(&self, callback:")
        });

        assert!(
            !has_attach_method,
            "BRIDGE-012: BackgroundSession should NOT have attach() method. \
             Remove pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>)."
        );
    }

    /// @step And there should be no detach method
    #[test]
    fn test_no_detach_method_in_background_session() {
        let source = read_session_manager_source();

        // Search for pub fn detach method (not in comments)
        let has_detach_method = source.lines().any(|line| {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("///") {
                return false;
            }
            // Check for method declaration
            trimmed.starts_with("pub fn detach(&self)")
        });

        assert!(
            !has_detach_method,
            "BRIDGE-012: BackgroundSession should NOT have detach() method. \
             Remove pub fn detach(&self)."
        );
    }

    /// @step And there should be no is_attached method
    #[test]
    fn test_no_is_attached_method_in_background_session() {
        let source = read_session_manager_source();

        // Search for pub fn is_attached method (not in comments)
        let has_is_attached_method = source.lines().any(|line| {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("///") {
                return false;
            }
            // Check for method declaration
            trimmed.starts_with("pub fn is_attached(&self)")
        });

        assert!(
            !has_is_attached_method,
            "BRIDGE-012: BackgroundSession should NOT have is_attached() method. \
             Remove pub fn is_attached(&self) -> bool."
        );
    }
}

mod no_per_session_napi_attachment_functions {
    use super::*;

    /// Feature: Global chunk callback NAPI for session-agnostic chunk emission
    /// Scenario: No per-session NAPI attachment functions
    ///
    /// @step When I inspect the NAPI module exports
    /// @step Then there should be no session_attach function
    #[test]
    fn test_no_session_attach_napi_export() {
        let declarations = read_napi_declarations();

        // Check TypeScript declarations for session_attach export
        // NAPI generates "export declare function" not "export function"
        let has_session_attach = declarations.contains("sessionAttach(sessionId:");

        assert!(
            !has_session_attach,
            "BRIDGE-012: NAPI should NOT export sessionAttach function. \
             Remove #[napi] pub fn session_attach from session_manager.rs."
        );
    }

    /// @step And there should be no session_detach function
    #[test]
    fn test_no_session_detach_napi_export() {
        let declarations = read_napi_declarations();

        // Check TypeScript declarations for session_detach export
        // NAPI generates "export declare function" not "export function"
        let has_session_detach = declarations.contains("sessionDetach(sessionId:");

        assert!(
            !has_session_detach,
            "BRIDGE-012: NAPI should NOT export sessionDetach function. \
             Remove #[napi] pub fn session_detach from session_manager.rs."
        );
    }

    /// @step And there should be a sessionSetGlobalChunkCallback function
    #[test]
    fn test_has_session_set_global_chunk_callback_napi_export() {
        let declarations = read_napi_declarations();

        // Check TypeScript declarations for sessionSetGlobalChunkCallback export
        // The declaration may span multiple lines with callback: on the next line
        let has_global_callback = declarations.contains("sessionSetGlobalChunkCallback(")
            && declarations.contains("callback:");

        assert!(
            has_global_callback,
            "BRIDGE-012: NAPI should export sessionSetGlobalChunkCallback function. \
             This is the new global callback registration function."
        );
    }
}

mod handle_output_uses_global_callback {
    use super::*;

    /// Additional verification for handle_output behavior
    ///
    /// Verifies that handle_output uses ONLY the global callback, not attached_callback.
    #[test]
    fn test_handle_output_does_not_use_attached_callback() {
        let source = read_session_manager_source();

        // Find the handle_output function and check it doesn't use attached_callback
        // We need to verify that handle_output does NOT reference attached_callback

        // This is a structural test - if attached_callback field doesn't exist,
        // handle_output can't use it. The other tests already verify the field is gone.
        // This test verifies there's no lingering reference.
        let has_attached_callback_usage_in_handle_output =
            source.contains("self.attached_callback");

        assert!(
            !has_attached_callback_usage_in_handle_output,
            "BRIDGE-012: handle_output should NOT use attached_callback. \
             All chunks should go through the global callback only."
        );
    }

    /// Verifies that handle_output does NOT check is_attached
    #[test]
    fn test_handle_output_does_not_check_is_attached() {
        let source = read_session_manager_source();

        let has_is_attached_check = source.contains("self.is_attached");

        assert!(
            !has_is_attached_check,
            "BRIDGE-012: handle_output should NOT check is_attached. \
             There should be no gating - all chunks go through the global callback."
        );
    }
}
