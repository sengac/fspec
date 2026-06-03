//! Stub for the `remove-aggregate-from-foundation` fspec command. See RPC-266 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-aggregate-from-foundation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-aggregate-from-foundation",
        work_unit: "RPC-266",
    })
}
