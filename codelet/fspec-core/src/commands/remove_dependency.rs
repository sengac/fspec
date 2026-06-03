//! Stub for the `remove-dependency` fspec command. See RPC-271 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-dependency.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-dependency",
        work_unit: "RPC-271",
    })
}
