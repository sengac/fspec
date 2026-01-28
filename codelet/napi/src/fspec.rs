//! NAPI bindings for FspecTool

use codelet_tools::FspecTool;
use napi::bindgen_prelude::*;

/// Call fspec command via NAPI callback pattern
/// 
/// The callback receives (command, args_json, project_root) and returns JSON result.
/// This enables TypeScript to import and execute actual fspec command modules.
#[napi(js_name = "callFspecCommand")]
pub fn call_fspec_command<T: Fn(String, String, String) -> Result<String>>(
    command: String,
    args_json: String, 
    project_root: String,
    callback: T,
) -> Result<String> {
    // Create FspecTool instance
    let tool = FspecTool::new();
    
    // Execute command via TypeScript callback
    // This calls the real fspec command modules in TypeScript
    tool.execute_via_callback(
        &command,
        &args_json,
        &project_root,
        |cmd, args, root| {
            callback(cmd, args, root).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
    ).map_err(|e| Error::new(Status::Unknown, e))
}