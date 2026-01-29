//! NAPI bindings for FspecTool
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
//! Implements the JS-controlled invocation pattern from rust-controlled.md

use napi::bindgen_prelude::*;

/// Call fspec command via JS-controlled invocation pattern
/// 
/// CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
/// Following rust-controlled.md pattern: JS explicitly passes the callback function
/// to Rust, which calls it immediately and returns the result.
/// 
/// TypeScript signature: 
/// ```typescript
/// function callFspecCommand(
///   command: string, 
///   argsJson: string, 
///   projectRoot: string, 
///   callback: (cmd: string, args: string, root: string) => string
/// ): string
/// ```
#[napi(js_name = "callFspecCommand")]
pub fn call_fspec_command(
    command: String,
    args_json: String, 
    project_root: String,
    callback: Function<(String, String, String), String>,
) -> Result<String> {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Execute the callback directly (JS-controlled invocation pattern)
    callback
        .call((command, args_json, project_root))
        .map_err(|e| Error::from_reason(e.to_string()))
}
