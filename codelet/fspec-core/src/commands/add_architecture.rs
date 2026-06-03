//! Stub for the `add-architecture` fspec command. See RPC-167 for the port work unit.
//! Original TypeScript implementation: src/commands/add-architecture.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-architecture",
        work_unit: "RPC-167",
    })
}
