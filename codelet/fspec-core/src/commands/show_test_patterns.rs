//! Stub for the `show-test-patterns` fspec command. See RPC-307 for the port work unit.
//! Original TypeScript implementation: src/commands/show-test-patterns.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-test-patterns",
        work_unit: "RPC-307",
    })
}
