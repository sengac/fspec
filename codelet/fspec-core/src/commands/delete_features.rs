//! Stub for the `delete-features` fspec command. See RPC-218 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-features-by-tag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-features",
        work_unit: "RPC-218",
    })
}
