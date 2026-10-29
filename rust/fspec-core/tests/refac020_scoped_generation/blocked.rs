use crate::support::*;
use serde_json::json;

#[test]
fn mapped_scenario_removal_blocks_all_selected_changes() -> TestResult {
    // @step Given a selected feature with new scenarios and complete or incomplete obsolete mapped lineage
    // @step When generation is previewed and applied
    // @step Then the entire plan is blocked without partial additions or lineage removal
    for lineage in [json!([mapping("first"), mapping("second")]), json!([{}])] {
        let temporary = fixture()?;
        let mut original = load(temporary.path())?;
        let mut obsolete = original["scenarios"][0].clone();
        obsolete["name"] = json!("Obsolete mapped thing");
        obsolete["testMappings"] = lineage.clone();
        original["scenarios"].as_array_mut().ok_or("scenario array")?.push(obsolete);
        save(temporary.path(), &original)?;
        add_new_scenario(temporary.path())?;
        let before = tree(temporary.path())?;
        let preview = request(temporary.path(), "generate-coverage",
            json!({"featureName": "thing", "dryRun": true}));
        assert_eq!(tree(temporary.path())?, before, "preview changed lineage {lineage}");
        let applied = request(temporary.path(), "generate-coverage", json!({"featureName": "thing"}));
        assert!(!applied.success, "complete or incomplete lineage must block: {lineage}: {applied:?}");
        assert_eq!(load(temporary.path())?, original);
        let mut after = tree(temporary.path())?;
        after.remove(&format!("{SIDECAR}.lock"));
        assert_eq!(after, before, "apply changed lineage {lineage}");
        let preview = successful_json(preview)?;
        assert_eq!(preview["success"], false);
        assert_eq!(preview["plan"]["status"], "blocked");
        assert_eq!(preview["plan"]["addedScenarios"], json!(["New thing"]));
        assert_eq!(preview["plan"]["removedScenarios"], json!(["Obsolete mapped thing"]));
        assert!(!preview["plan"]["blockedReasons"].as_array().ok_or("blocked reasons")?.is_empty());
    }
    Ok(())
}

#[test]
fn invalid_or_unreadable_existing_metadata_is_never_recreated() -> TestResult {
    // @step Given invalid malformed or unreadable selected metadata
    // @step When generation is previewed and applied
    // @step Then the blocked metadata is never recreated and unrelated bytes remain unchanged
    for invalid in ["{bad JSON", "[]", "{\"scenarios\":{}}", "{\"scenarios\":[true]}",
        "{\"scenarios\":[{\"name\":\"Thing does the thing\",\"testMappings\":{}}]}", "directory"] {
        let temporary = fixture()?;
        add_new_scenario(temporary.path())?;
        if invalid == "directory" {
            std::fs::remove_file(temporary.path().join(SIDECAR))?;
            std::fs::create_dir(temporary.path().join(SIDECAR))?;
            write(temporary.path(), &format!("{SIDECAR}/lineage-receipt"), "preserve me")?;
        } else {
            write(temporary.path(), SIDECAR, invalid)?;
        }
        let before = tree(temporary.path())?;
        let preview = request(temporary.path(), "generate-coverage",
            json!({"featureName": "thing", "dryRun": true}));
        assert_eq!(tree(temporary.path())?, before, "preview changed {invalid}");
        let result = request(temporary.path(), "generate-coverage", json!({"featureName": "thing"}));
        assert!(!result.success, "must not recreate {invalid}: {result:?}");
        let mut after = tree(temporary.path())?;
        after.remove(&format!("{SIDECAR}.lock"));
        assert_eq!(after, before, "apply changed {invalid}");
        let preview = successful_json(preview)?;
        assert_eq!(preview["plan"]["status"], "blocked", "{invalid}");
        assert_eq!(preview["success"], false, "{invalid}");
        assert!(!preview["plan"]["blockedReasons"].as_array().ok_or("blocked reasons")?.is_empty());
    }
    Ok(())
}
