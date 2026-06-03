//! Stub for the `delete-scenarios` fspec command. See RPC-220 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-scenarios-by-tag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-scenarios",
        work_unit: "RPC-220",
    })
}
