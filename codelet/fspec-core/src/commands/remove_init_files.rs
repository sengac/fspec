//! Stub for the `remove-init-files` fspec command. See RPC-276 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-init-files.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-init-files",
        work_unit: "RPC-276",
    })
}
