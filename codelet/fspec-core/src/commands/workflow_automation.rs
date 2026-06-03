//! Stub for the `workflow-automation` fspec command. See RPC-326 for the port work unit.
//! Original TypeScript implementation: src/commands/workflow-automation.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "workflow-automation",
        work_unit: "RPC-326",
    })
}
