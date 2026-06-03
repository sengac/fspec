//! Stub for the `export-work-units` fspec command. See RPC-229 for the port work unit.
//! Original TypeScript implementation: src/commands/export-work-units.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "export-work-units",
        work_unit: "RPC-229",
    })
}
