//! Stub for the `tag-stats` fspec command. See RPC-310 for the port work unit.
//! Original TypeScript implementation: src/commands/tag-stats.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "tag-stats",
        work_unit: "RPC-310",
    })
}
