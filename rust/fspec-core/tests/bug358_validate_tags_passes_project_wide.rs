//! BUG-358 / COV-065 — validate-tags passes for every feature file.
//!
//! Feature: spec/features/validate-tags-passes-project-wide.feature

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn validate_tags_reports_no_invalid_feature_files_in_the_repository() -> TestResult {
    // @step Given the repository's spec directory
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("the repository root is above rust/fspec-core")?;
    // @step When the validate-tags command runs over every feature file
    let output = codelet_fspec_core::commands::validate_tags::run("{}", repository).await?;
    let result: serde_json::Value = serde_json::from_str(&output)?;
    // @step Then no feature file has a tag violation
    let invalid: Vec<&serde_json::Value> = result["results"]
        .as_array()
        .ok_or("validate-tags returned no results")?
        .iter()
        .filter(|file| file["valid"] == false)
        .collect();
    assert_eq!(result["invalidCount"], 0, "invalid files: {invalid:#?}");
    assert!(result["validCount"].as_u64().ok_or("no validCount")? > 0);
    Ok(())
}
