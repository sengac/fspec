//! Stub for the `delete-epic` fspec command. See RPC-217 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-epic.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-epic",
        work_unit: "RPC-217",
    })
}
