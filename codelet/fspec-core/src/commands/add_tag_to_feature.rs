//! Stub for the `add-tag-to-feature` fspec command. See RPC-193 for the port work unit.
//! Original TypeScript implementation: src/commands/add-tag-to-feature.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-tag-to-feature",
        work_unit: "RPC-193",
    })
}
