//! Stub for the `get-scenarios` fspec command. See RPC-237 for the port work unit.
//! Original TypeScript implementation: src/commands/get-scenarios.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "get-scenarios",
        work_unit: "RPC-237",
    })
}
