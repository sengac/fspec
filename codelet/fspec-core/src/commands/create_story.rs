//! Stub for the `create-story` fspec command. See RPC-214 for the port work unit.
//! Original TypeScript implementation: src/commands/create-story.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "create-story",
        work_unit: "RPC-214",
    })
}
