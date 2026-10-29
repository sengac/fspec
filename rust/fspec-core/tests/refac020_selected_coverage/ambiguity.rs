use crate::support::*;
use serde_json::json;

#[test]
fn ambiguous_same_name_scenarios_reject_indexed_mutations_without_writes() -> TestResult {
    // @step Given two raw coverage scenarios with the same name and distinct metadata
    let temporary = fixture()?;
    let mut ambiguous = load(temporary.path())?;
    let mut duplicate = ambiguous["scenarios"][0].clone();
    duplicate["unknownScenario"]["duplicateReceipt"] = json!([7, 3, 7]);
    duplicate["testMappings"][0]["lineage"] = json!("second-same-name-scenario");
    ambiguous["scenarios"].as_array_mut().ok_or("scenario array")?.push(duplicate);
    save(temporary.path(), &ambiguous)?;
    let before = tree(temporary.path())?;
    let args = indexed_link(temporary.path(), 1, "src/second.rs")?;

    // @step When indexed linking and unlinking select that ambiguous name
    // @step Then neither scenario is chosen and exact sidecar bytes remain unchanged
    for command in ["link-coverage", "unlink-coverage"] {
        let mut selected_args = args.clone();
        if command == "unlink-coverage" {
            selected_args.as_object_mut().ok_or("argument object")?.remove("implFile");
            selected_args.as_object_mut().ok_or("argument object")?.remove("implLines");
        }
        let result = request(temporary.path(), command, selected_args);
        assert!(!result.success, "{command} arbitrarily selected a same-name scenario: {result:?}");
        assert_eq!(load(temporary.path())?, ambiguous);
        let mut after = tree(temporary.path())?;
        after.remove(&format!("{SIDECAR}.lock"));
        assert_eq!(after, before, "{command} rewrote ambiguous scenario bytes");
    }
    Ok(())
}
