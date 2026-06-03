//! Stub for the `remove-hook` fspec command. See RPC-275 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-hook.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-hook",
        work_unit: "RPC-275",
    })
}
