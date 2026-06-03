//! Stub for the `show-acceptance-criteria` fspec command. See RPC-299 for the port work unit.
//! Original TypeScript implementation: src/commands/show-acceptance-criteria.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-acceptance-criteria",
        work_unit: "RPC-299",
    })
}
