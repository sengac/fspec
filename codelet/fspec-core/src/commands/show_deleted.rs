//! Stub for the `show-deleted` fspec command. See RPC-301 for the port work unit.
//! Original TypeScript implementation: src/commands/show-deleted.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-deleted",
        work_unit: "RPC-301",
    })
}
