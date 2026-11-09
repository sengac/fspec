use crate::process::{commit_while_locked, hold_transaction, Worker};
use crate::support::*;
use serde_json::json;

#[test]
fn competing_selected_mutations_allow_one_fresh_hash_and_reject_the_stale_one() -> TestResult {
    // @step Given two processes selecting the same scenario revision
    let temporary = fixture()?;
    let original = load(temporary.path())?;
    let original_bytes = std::fs::read(temporary.path().join(SIDECAR))?;

    // @step And an externally held transaction lock for equivalent feature spellings
    let guard = hold_transaction(temporary.path())?;
    let mut equivalent_args = indexed_link(temporary.path(), 1, "src/third.rs")?;
    equivalent_args["featureName"] = json!("thing.feature");
    let mut first = Worker::start(
        temporary.path(),
        "first",
        "link-coverage",
        indexed_link(temporary.path(), 1, "src/second.rs")?,
    )?;
    let mut second = Worker::start(temporary.path(), "second", "link-coverage", equivalent_args)?;

    // @step When both attempt distinct implementation additions
    first.begin()?;
    second.begin()?;

    // @step Then both remain blocked without changing sidecar bytes while the lock is held
    first.assert_waiting()?;
    second.assert_waiting()?;
    first.assert_pending()?;
    second.assert_pending()?;
    assert_eq!(
        std::fs::read(temporary.path().join(SIDECAR))?,
        original_bytes
    );
    drop(guard);
    let outcomes = [first.finish()?, second.finish()?];

    // @step And exactly one commits and the other rejects its stale hash without lost updates after release
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| result["success"] == true)
            .count(),
        1,
        "{outcomes:?}"
    );
    let failed = outcomes
        .iter()
        .find(|result| result["success"] == false)
        .ok_or("one stale failure")?;
    let diagnostic = failed["error"]
        .as_str()
        .ok_or("stale diagnostic")?
        .to_lowercase();
    assert!(
        diagnostic.contains("stale") || diagnostic.contains("hash"),
        "{diagnostic}"
    );
    let actual = load(temporary.path())?;
    assert_eq!(
        actual["scenarios"][0]["testMappings"][0],
        original["scenarios"][0]["testMappings"][0]
    );
    let mut expected = original["scenarios"].clone();
    let winner = if outcomes[0]["success"] == true {
        "src/second.rs"
    } else {
        "src/third.rs"
    };
    expected[0]["testMappings"][1]["implMappings"]
        .as_array_mut()
        .ok_or("implementation array")?
        .push(
            json!({"file": winner, "lines": [1, 2], "firstLine": "fn thing() {", "lastLine": "}"}),
        );
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    let successful = outcomes
        .iter()
        .find(|result| result["success"] == true)
        .ok_or("one successful result")?;
    let envelope: serde_json::Value =
        serde_json::from_str(successful["data"].as_str().ok_or("result data")?)?;
    assert_eq!(
        envelope["scenarioHash"],
        scenario_hash(&actual["scenarios"][0])?
    );
    Ok(())
}

#[test]
fn legacy_link_and_unlink_wait_for_the_shared_sidecar_transaction() -> TestResult {
    // @step Given an externally held stable sidecar transaction lock
    // @step When legacy link and unlink run in separate processes using equivalent feature spellings
    // @step Then each waits and reads the latest committed mappings after lock release
    for (command, feature_name) in [
        ("link-coverage", "thing"),
        ("link-coverage", "thing.feature"),
        ("unlink-coverage", "thing"),
        ("unlink-coverage", "thing.feature"),
    ] {
        let temporary = fixture()?;
        let original = std::fs::read(temporary.path().join(SIDECAR))?;
        let guard = hold_transaction(temporary.path())?;
        let mut args =
            json!({"featureName": feature_name, "scenario": SCENARIO, "testFile": TEST_FILE});
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
            expected[0]["testMappings"][0]["implMappings"]
                .as_array_mut()
                .ok_or("implementation array")?
                .push(json!({"file": "src/second.rs", "lines": [1, 2], "firstLine": "fn thing() {", "lastLine": "}"}));
        } else {
            expected[0]["testMappings"]
                .as_array_mut()
                .ok_or("mapping array")?
                .remove(0);
        }
        assert_eq!(
            actual["scenarios"], expected,
            "{command} lost a committed mapping or read before acquiring the guard"
        );
    }
    Ok(())
}

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
        assert_eq!(
            result["success"], true,
            "feature={feature_name:?}: {result:?}"
        );
        let actual = load(temporary.path())?;
        assert_eq!(
            actual["scenarios"][0], latest["scenarios"][0],
            "feature={feature_name:?} lost a committed mapping or read before acquiring the guard"
        );
        assert_eq!(
            actual["scenarios"][1],
            json!({"name": "New thing", "testMappings": []})
        );
        assert_eq!(
            actual["scenarios"]
                .as_array()
                .ok_or("scenario array")?
                .len(),
            2
        );
    }
    Ok(())
}
