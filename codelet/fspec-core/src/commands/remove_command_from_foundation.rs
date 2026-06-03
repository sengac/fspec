//! Stub for the `remove-command-from-foundation` fspec command. See RPC-270 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-command-from-foundation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-command-from-foundation",
        work_unit: "RPC-270",
    })
}
