//! Stub for the `generate-scenarios` fspec command. See RPC-234 for the port work unit.
//! Original TypeScript implementation: src/commands/generate-scenarios.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "generate-scenarios",
        work_unit: "RPC-234",
    })
}
