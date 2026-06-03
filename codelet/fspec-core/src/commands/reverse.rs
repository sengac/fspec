//! Stub for the `reverse` fspec command. See RPC-294 for the port work unit.
//! Original TypeScript implementation: src/commands/reverse.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "reverse",
        work_unit: "RPC-294",
    })
}
