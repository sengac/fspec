//! Stub for the `add-architecture-note` fspec command. See RPC-168 for the port work unit.
//! Original TypeScript implementation: src/commands/add-architecture-note.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-architecture-note",
        work_unit: "RPC-168",
    })
}
