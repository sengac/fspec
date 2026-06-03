//! Stub for the `update-prefix` fspec command. See RPC-313 for the port work unit.
//! Original TypeScript implementation: src/commands/update-prefix.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-prefix",
        work_unit: "RPC-313",
    })
}
