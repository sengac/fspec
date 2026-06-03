//! Stub for the `add-virtual-hook` fspec command. See RPC-195 for the port work unit.
//! Original TypeScript implementation: src/commands/add-virtual-hook.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-virtual-hook",
        work_unit: "RPC-195",
    })
}
