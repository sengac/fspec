//! Stub for the `search-scenarios` fspec command. See RPC-297 for the port work unit.
//! Original TypeScript implementation: src/commands/search-scenarios.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "search-scenarios",
        work_unit: "RPC-297",
    })
}
