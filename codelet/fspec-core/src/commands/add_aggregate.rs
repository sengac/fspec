//! Stub for the `add-aggregate` fspec command. See RPC-165 for the port work unit.
//! Original TypeScript implementation: src/commands/add-aggregate.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-aggregate",
        work_unit: "RPC-165",
    })
}
