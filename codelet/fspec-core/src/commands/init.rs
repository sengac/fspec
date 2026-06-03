//! Stub for the `init` fspec command. See RPC-239 for the port work unit.
//! Original TypeScript implementation: src/commands/init.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "init",
        work_unit: "RPC-239",
    })
}
