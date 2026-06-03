//! Stub for the `show-foundation` fspec command. See RPC-305 for the port work unit.
//! Original TypeScript implementation: src/commands/show-foundation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-foundation",
        work_unit: "RPC-305",
    })
}
