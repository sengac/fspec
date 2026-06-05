//! Stub for the `list-attachments` fspec command. See RPC-241 for the port work unit.
//! Original TypeScript implementation: src/commands/list-attachments.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-attachments",
        work_unit: "RPC-241",
    })
}
