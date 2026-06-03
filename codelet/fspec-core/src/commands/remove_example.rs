//! Stub for the `remove-example` fspec command. See RPC-273 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-example.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-example",
        work_unit: "RPC-273",
    })
}
