//! Stub for the `add-hook` fspec command. See RPC-184 for the port work unit.
//! Original TypeScript implementation: src/commands/add-hook.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-hook",
        work_unit: "RPC-184",
    })
}
