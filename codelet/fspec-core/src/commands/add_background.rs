//! Stub for the `add-background` fspec command. See RPC-171 for the port work unit.
//! Original TypeScript implementation: src/commands/add-background.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-background",
        work_unit: "RPC-171",
    })
}
