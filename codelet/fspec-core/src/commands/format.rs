//! Stub for the `format` fspec command. See RPC-230 for the port work unit.
//! Original TypeScript implementation: src/commands/format.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "format",
        work_unit: "RPC-230",
    })
}
