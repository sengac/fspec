//! Stub for the `add-example` fspec command. See RPC-181 for the port work unit.
//! Original TypeScript implementation: src/commands/add-example.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-example",
        work_unit: "RPC-181",
    })
}
