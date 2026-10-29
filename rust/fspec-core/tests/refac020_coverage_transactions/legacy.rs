use crate::process::{commit_while_locked, hold_transaction, Worker};
use crate::support::*;
use serde_json::json;

#[test]
fn legacy_link_and_unlink_wait_for_the_shared_sidecar_transaction() -> TestResult {
    // @step Given an externally held stable sidecar transaction lock
    // @step When legacy link and unlink run in separate processes using equivalent feature spellings
    // @step Then each waits and reads the latest committed mappings after lock release
    for (command, feature_name) in [
        ("link-coverage", "thing"), ("link-coverage", "thing.feature"),
        ("unlink-coverage", "thing"), ("unlink-coverage", "thing.feature"),
    ] {
        let temporary = fixture()?;
        let original = std::fs::read(temporary.path().join(SIDECAR))?;
        let guard = hold_transaction(temporary.path())?;
        let mut args = json!({"featureName": feature_name, "scenario": SCENARIO, "testFile": TEST_FILE});
        if command == "link-coverage" {
            args["implFile"] = json!("src/second.rs");
            args["implLines"] = json!("1,2");
        }
        let mut worker = Worker::start(temporary.path(), command, command, args)?;
        worker.begin()?;
        worker.assert_waiting()?;
        assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
        let latest = commit_while_locked(temporary.path())?;
        drop(guard);
        let result = worker.finish()?;
        assert_eq!(result["success"], true, "{command}: {result:?}");
        let actual = load(temporary.path())?;
        let mut expected = latest["scenarios"].clone();
        if command == "link-coverage" {
            expected[0]["testMappings"][0]["implMappings"].as_array_mut().ok_or("implementation array")?
                .push(json!({"file": "src/second.rs", "lines": [1, 2]}));
        } else {
            expected[0]["testMappings"].as_array_mut().ok_or("mapping array")?.remove(0);
        }
        assert_eq!(actual["scenarios"], expected, "{command} lost a committed mapping or read before acquiring the guard");
    }
    Ok(())
}
