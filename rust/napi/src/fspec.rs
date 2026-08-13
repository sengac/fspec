//! NAPI bindings for FspecTool
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
//! Implements the JS-controlled invocation pattern from rust-controlled.md
//!
//! ## Architecture
//!
//! The FspecTool callback flow works as follows:
//!
//! 1. TypeScript calls `callFspecCommand(command, argsJson, projectRoot, callback)`
//! 2. Rust receives these parameters and immediately calls the callback with all three args
//! 3. The TypeScript callback executes the actual fspec command logic
//! 4. The callback returns a JSON string result back to Rust
//! 5. Rust returns this result to the original TypeScript caller
//!
//! This is the synchronous JS-controlled invocation pattern. For async agent tool
//! usage, see the `FspecCommandRequest` / `FspecCommandResult` StreamChunk flow
//! in session_manager.rs which handles the async callback via channels.

use napi::bindgen_prelude::*;

/// Call fspec command via JS-controlled invocation pattern
///
/// CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
/// Following rust-controlled.md pattern: JS explicitly passes the callback function
/// to Rust, which calls it immediately and returns the result.
///
/// ## TypeScript signature
///
/// ```typescript
/// function callFspecCommand(
///   command: string,
///   argsJson: string,
///   projectRoot: string,
///   callback: (cmd: string, args: string, root: string) => string
/// ): string
/// ```
///
/// ## Example
///
/// ```typescript
/// const result = callFspecCommand(
///   'list-work-units',
///   '{"status":"backlog"}',
///   '/path/to/project',
///   (command, argsJson, projectRoot) => {
///     // Execute command and return JSON result
///     return JSON.stringify({ success: true, data: { workUnits: [] } });
///   }
/// );
/// ```
///
/// ## NAPI-RS Note
///
/// For multiple callback arguments, we use `FnArgs<(...)>` wrapper and call with `.into()`
/// to properly destructure the tuple into separate JavaScript function parameters.
#[napi(js_name = "callFspecCommand")]
pub fn call_fspec_command(
    command: String,
    args_json: String,
    project_root: String,
    callback: Function<FnArgs<(String, String, String)>, String>,
) -> Result<String> {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Execute the callback directly (JS-controlled invocation pattern)
    // The .into() converts the tuple to FnArgs for proper multi-argument destructuring
    callback
        .call((command, args_json, project_root).into())
        .map_err(|e| Error::from_reason(e.to_string()))
}
