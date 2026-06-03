//! Stub for the `add-policy` fspec command. See RPC-187 for the port work unit.
//! Original TypeScript implementation: src/commands/add-policy.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-policy",
        work_unit: "RPC-187",
    })
}
