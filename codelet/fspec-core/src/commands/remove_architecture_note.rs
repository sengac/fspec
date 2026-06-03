//! Stub for the `remove-architecture-note` fspec command. See RPC-267 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-architecture-note.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-architecture-note",
        work_unit: "RPC-267",
    })
}
