//! Stub for the `search-implementation` fspec command. See RPC-296 for the port work unit.
//! Original TypeScript implementation: src/commands/search-implementation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "search-implementation",
        work_unit: "RPC-296",
    })
}
