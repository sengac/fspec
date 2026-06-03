//! Stub for the `configure-tools` fspec command. See RPC-208 for the port work unit.
//! Original TypeScript implementation: src/commands/configure-tools.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "configure-tools",
        work_unit: "RPC-208",
    })
}
