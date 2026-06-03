//! Stub for the `add-aggregate-to-foundation` fspec command. See RPC-166 for the port work unit.
//! Original TypeScript implementation: src/commands/add-aggregate-to-foundation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-aggregate-to-foundation",
        work_unit: "RPC-166",
    })
}
