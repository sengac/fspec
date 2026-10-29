use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::support::*;

#[test]
#[ignore = "subprocess-only transaction probe; parent acceptance tests supply isolated inputs"]
fn transaction_worker_process() -> TestResult {
    let root = PathBuf::from(std::env::var_os("REFAC020_WORKER_ROOT").ok_or("worker root")?);
    let ipc = PathBuf::from(std::env::var_os("REFAC020_WORKER_IPC").ok_or("worker ipc")?);
    let command = std::env::var("REFAC020_WORKER_COMMAND")?;
    let args: Value = serde_json::from_str(&std::env::var("REFAC020_WORKER_ARGS")?)?;
    std::fs::write(ipc.join("ready"), b"ready")?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ipc.join("go").exists() {
        if Instant::now() >= deadline {
            return Err("parent did not release worker start gate".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::write(ipc.join("entered"), b"dispatch entered")?;
    let result = request(&root, &command, args);
    std::fs::write(ipc.join("result.json"), serde_json::to_vec(&json!({
        "success": result.success, "data": result.data, "error": result.error
    }))?)?;
    Ok(())
}
