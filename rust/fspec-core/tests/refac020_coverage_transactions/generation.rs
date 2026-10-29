use crate::process::{commit_while_locked, hold_transaction, Worker};
use crate::support::*;
use serde_json::json;

#[test]
fn selected_and_global_generation_wait_then_preserve_latest_participating_write() -> TestResult {
    // @step Given an externally held stable sidecar transaction lock
    // @step When selected and global generation run in separate processes using equivalent feature spellings
    // @step Then each waits and preserves the latest mappings after lock release
    for feature_name in [Some("thing"), Some("thing.feature"), None] {
        let temporary = fixture()?;
        add_new_scenario(temporary.path())?;
        let original = std::fs::read(temporary.path().join(SIDECAR))?;
        let guard = hold_transaction(temporary.path())?;
        let args = match feature_name {
            Some(name) => json!({"featureName": name}),
            None => json!({}),
        };
        let mut worker = Worker::start(temporary.path(), "generator", "generate-coverage", args)?;
        worker.begin()?;
        worker.assert_waiting()?;
        assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
        let latest = commit_while_locked(temporary.path())?;
        drop(guard);
        let result = worker.finish()?;
        assert_eq!(result["success"], true, "feature={feature_name:?}: {result:?}");
        let actual = load(temporary.path())?;
        assert_eq!(actual["scenarios"][0], latest["scenarios"][0], "feature={feature_name:?} lost a committed mapping or read before acquiring the guard");
        assert_eq!(actual["scenarios"][1], json!({"name": "New thing", "testMappings": []}));
        assert_eq!(actual["scenarios"].as_array().ok_or("scenario array")?.len(), 2);
    }
    Ok(())
}
