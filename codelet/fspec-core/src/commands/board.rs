//! Stub for the `board` fspec command. See RPC-199 for the port work unit.
//! Original TypeScript implementation: src/commands/display-board.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "board",
        work_unit: "RPC-199",
    })
}
