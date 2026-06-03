//! Stub for the `list-features` fspec command. See RPC-245 for the port work unit.
//! Original TypeScript implementation: src/commands/list-features.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-features",
        work_unit: "RPC-245",
    })
}
