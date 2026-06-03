//! Stub for the `restore-architecture-note` fspec command. See RPC-287 for the port work unit.
//! Original TypeScript implementation: src/commands/restore-architecture-note.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "restore-architecture-note",
        work_unit: "RPC-287",
    })
}
