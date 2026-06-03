//! Stub for the `create-prefix` fspec command. See RPC-213 for the port work unit.
//! Original TypeScript implementation: src/commands/create-prefix.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "create-prefix",
        work_unit: "RPC-213",
    })
}
