//! Stub for the `validate-work-units` fspec command. See RPC-325 for the port work unit.
//! Original TypeScript implementation: src/commands/validate-work-units.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "validate-work-units",
        work_unit: "RPC-325",
    })
}
