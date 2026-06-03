//! Stub for the `add-bounded-context` fspec command. See RPC-172 for the port work unit.
//! Original TypeScript implementation: src/commands/add-bounded-context.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-bounded-context",
        work_unit: "RPC-172",
    })
}
