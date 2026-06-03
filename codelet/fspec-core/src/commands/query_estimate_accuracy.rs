//! Stub for the `query-estimate-accuracy` fspec command. See RPC-258 for the port work unit.
//! Original TypeScript implementation: src/commands/query-estimate-accuracy.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-estimate-accuracy",
        work_unit: "RPC-258",
    })
}
