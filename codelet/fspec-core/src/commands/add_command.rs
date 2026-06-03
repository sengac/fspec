//! Stub for the `add-command` fspec command. See RPC-174 for the port work unit.
//! Original TypeScript implementation: src/commands/add-command.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-command",
        work_unit: "RPC-174",
    })
}
