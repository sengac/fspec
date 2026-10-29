use crate::process::{hold_transaction, Worker};
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
    let mut first = Worker::start(temporary.path(), "first", "link-coverage",
        indexed_link(temporary.path(), 1, "src/second.rs")?)?;
    let mut second = Worker::start(temporary.path(), "second", "link-coverage",
        equivalent_args)?;

    // @step When both attempt distinct implementation additions
    first.begin()?;
    second.begin()?;

    // @step Then both remain blocked without changing sidecar bytes while the lock is held
    first.assert_waiting()?;
    second.assert_waiting()?;
    first.assert_pending()?;
    second.assert_pending()?;
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original_bytes);
    drop(guard);
    let outcomes = [first.finish()?, second.finish()?];

    // @step And exactly one commits and the other rejects its stale hash without lost updates after release
    assert_eq!(outcomes.iter().filter(|result| result["success"] == true).count(), 1, "{outcomes:?}");
    let failed = outcomes.iter().find(|result| result["success"] == false).ok_or("one stale failure")?;
    let diagnostic = failed["error"].as_str().ok_or("stale diagnostic")?.to_lowercase();
    assert!(diagnostic.contains("stale") || diagnostic.contains("hash"), "{diagnostic}");
    let actual = load(temporary.path())?;
    assert_eq!(actual["scenarios"][0]["testMappings"][0], original["scenarios"][0]["testMappings"][0]);
    let mut expected = original["scenarios"].clone();
    let winner = if outcomes[0]["success"] == true { "src/second.rs" } else { "src/third.rs" };
    expected[0]["testMappings"][1]["implMappings"].as_array_mut().ok_or("implementation array")?
        .push(json!({"file": winner, "lines": [1, 2]}));
    assert_eq!(actual["scenarios"], expected);
    assert_eq!(actual["unknownTop"], original["unknownTop"]);
    let successful = outcomes.iter().find(|result| result["success"] == true).ok_or("one successful result")?;
    let envelope: serde_json::Value = serde_json::from_str(successful["data"].as_str().ok_or("result data")?)?;
    assert_eq!(envelope["scenarioHash"], scenario_hash(&actual["scenarios"][0])?);
    Ok(())
}
