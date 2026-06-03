//! Stub for the `show-feature` fspec command. See RPC-304 for the port work unit.
//! Original TypeScript implementation: src/commands/show-feature.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-feature",
        work_unit: "RPC-304",
    })
}
