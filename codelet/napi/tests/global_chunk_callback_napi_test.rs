/**
 * Feature: spec/features/global-chunk-callback-napi.feature
 *
 * Tests for global chunk callback NAPI architecture.
 * Rust exposes a single global callback via NAPI that TypeScript registers once at startup.
 * ALL chunks from ALL sessions go through this ONE callback with signature (session_id, chunk).
 * Rust has ZERO knowledge of which session is active/attached - it's a pure emitter.
 * This replaces the per-session attach()/detach() pattern completely.
 */

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
        let has_global_callback_export = declarations.contains("sessionSetGlobalChunkCallback(callback:");
        
        assert!(
            has_global_callback_export,
            "BRIDGE-012: NAPI should export sessionSetGlobalChunkCallback for TypeScript to call at startup."
        );
    }

    #[test]
    fn test_global_callback_accepts_callback_parameter() {
        let declarations = read_napi_declarations();
        
        // Verify the callback parameter accepts a function
        let has_callback_param = declarations.contains("sessionSetGlobalChunkCallback(callback: (");
        
        assert!(
            has_callback_param,
            "BRIDGE-012: sessionSetGlobalChunkCallback should accept a callback function parameter."
        );
    }

    #[test]
    fn test_global_callback_returns_void() {
        let source = read_session_manager_source();
        
        // Verify sessionSetGlobalChunkCallback returns Result<()> in Rust
        let has_correct_signature = source.contains("pub fn session_set_global_chunk_callback(callback:") 
            && source.contains("-> Result<()>");
        
        assert!(
            has_correct_signature,
            "BRIDGE-012: session_set_global_chunk_callback should return Result<()>."
        );
    }
}

mod emit_chunk_with_session_id_through_global_callback {
    use super::*;

    /// Feature: Global chunk callback NAPI for session-agnostic chunk emission
    /// Scenario: Emit chunk with session_id through global callback
    ///
    /// @step Given a global chunk callback is registered
    /// @step And a session exists with id "session-abc"
    /// @step When the session emits a TextDelta chunk via handle_output
    /// @step Then the global callback should be invoked with session_id "session-abc"
    /// @step And the global callback should receive the TextDelta chunk
    #[test]
    fn test_handle_output_uses_global_callback() {
        let source = read_session_manager_source();
        
        // Verify handle_output calls the global callback
        // The code should contain: GLOBAL_CHUNK_CALLBACK.get() and global_cb.call(self.id.to_string()
        let has_global_callback_usage = source.contains("GLOBAL_CHUNK_CALLBACK.get()");
        
        assert!(
            has_global_callback_usage,
            "BRIDGE-012: handle_output should use GLOBAL_CHUNK_CALLBACK to emit chunks."
        );
    }

    #[test]
    fn test_global_callback_receives_session_id() {
        let source = read_session_manager_source();
        
        // Verify the global callback is called with session_id (self.id.to_string())
        let has_session_id_in_call = source.contains("global_cb.call(self.id.to_string()");
        
        assert!(
            has_session_id_in_call,
            "BRIDGE-012: Global callback should be called with session_id (self.id.to_string())."
        );
    }

    #[test]
    fn test_global_callback_receives_chunk() {
        let source = read_session_manager_source();
        
        // Verify the chunk is passed to global callback: global_cb.call(session_id, chunk)
        let has_chunk_in_call = source.contains("global_cb.call(self.id.to_string(), chunk");
        
        assert!(
            has_chunk_in_call,
            "BRIDGE-012: Global callback should be called with the chunk."
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
    /// @step Then both chunks should go through the same global callback
    /// @step And each chunk should have its respective session_id
    #[test]
    fn test_global_callback_is_static_singleton() {
        let source = read_session_manager_source();
        
        // Verify GLOBAL_CHUNK_CALLBACK is a static OnceCell
        let has_static_global_callback = source.contains("static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback>");
        
        assert!(
            has_static_global_callback,
            "BRIDGE-012: GLOBAL_CHUNK_CALLBACK should be a static OnceCell, shared by all sessions."
        );
    }

    #[test]
    fn test_each_session_passes_own_id_to_callback() {
        let source = read_session_manager_source();
        
        // Each session uses self.id.to_string() when calling the global callback
        // This ensures each chunk carries its session's ID
        let has_self_id_pattern = source.contains("self.id.to_string()");
        
        assert!(
            has_self_id_pattern,
            "BRIDGE-012: Each session should pass self.id.to_string() to the global callback."
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
            trimmed.starts_with("is_attached: AtomicBool") || trimmed.starts_with("is_attached: bool")
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
        // NAPI generates "export declare function" not "export function"
        let has_global_callback = declarations.contains("sessionSetGlobalChunkCallback(callback:");
        
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
