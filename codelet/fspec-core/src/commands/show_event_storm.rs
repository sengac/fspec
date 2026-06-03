//! Stub for the `show-event-storm` fspec command. See RPC-303 for the port work unit.
//! Original TypeScript implementation: src/commands/show-event-storm.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-event-storm",
        work_unit: "RPC-303",
    })
}
