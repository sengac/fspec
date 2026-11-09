//! BUG-435 — the temporal gate checks coverage-mapped test files in any
//! language, not only TypeScript `__tests__` files.
//!
//! Feature: spec/features/temporal-gate-mapped-test-files.feature

use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

/// STORY-1 entered testing one hour ago.
fn story_in_testing(root: &Path) -> TestResult {
    let entered = chrono_free_iso(SystemTime::now() - Duration::from_secs(3_600))?;
    let data = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": { "STORY-1": {
            "id": "STORY-1", "title": "Gate", "type": "story", "status": "testing",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z",
            "stateHistory": [ { "state": "testing", "timestamp": entered } ]
        }},
        "states": { "backlog": [], "specifying": [], "testing": ["STORY-1"], "implementing": [],
                    "validating": [], "done": [], "blocked": [] }
    });
    fs::create_dir_all(root.join("spec"))?;
    fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_string_pretty(&data)?,
    )?;
    Ok(())
}

/// `YYYY-MM-DDTHH:MM:SS.000Z` for `time`.
fn chrono_free_iso(time: SystemTime) -> Result<String, Box<dyn Error>> {
    let secs = time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    ))
}

/// A feature tagged @STORY-1 whose coverage maps `test_file`, which holds
/// `content` and the scenario's @step comment, and was last modified two
/// hours ago (before testing began).
fn mapped_old_test_file(root: &Path, test_file: &str, content: &str) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features/gate.feature"),
        "@STORY-1\nFeature: Gate\n\n  Scenario: Gate\n    Given a gate\n",
    )?;
    let coverage = json!({
        "scenarios": [ { "name": "Gate", "testMappings": [ { "file": test_file, "lines": "1-2" } ] } ]
    });
    fs::write(
        root.join("spec/features/gate.feature.coverage"),
        serde_json::to_string_pretty(&coverage)?,
    )?;
    let path = root.join(test_file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("{content}// @step Given a gate\n"))?;
    fs::File::options()
        .write(true)
        .open(&path)?
        .set_modified(SystemTime::now() - Duration::from_secs(7_200))?;
    Ok(())
}

/// When the user moves "STORY-1" to implementing.
fn move_to_implementing(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({ "workUnitId": "STORY-1", "status": "implementing" }).to_string(),
        project_root: root.to_path_buf(),
    })
}

#[test]
fn a_mapped_test_file_written_before_testing_blocks_the_move_to_implementing() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given story "STORY-1" is in testing since a known time
    story_in_testing(root)?;
    // @step And the linked feature's coverage maps "rust/tests/gate.rs"
    // @step And "rust/tests/gate.rs" mentions "STORY-1" and was last modified before STORY-1 entered testing
    mapped_old_test_file(root, "rust/tests/gate.rs", "// STORY-1 gate test\n")?;
    // @step When the user moves "STORY-1" to implementing
    let result = move_to_implementing(root);
    // @step Then the move fails with a temporal ordering violation naming "rust/tests/gate.rs"
    let error = result.error.as_deref().unwrap_or_default();
    if result.success
        || !error.contains("temporal ordering violation")
        || !error.contains("rust/tests/gate.rs")
    {
        return Err(format!("the old mapped test must block the move: {result:?}").into());
    }
    Ok(())
}

#[test]
fn a_mapped_test_file_that_does_not_mention_the_work_unit_is_not_checked() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given story "STORY-1" is in testing since a known time
    story_in_testing(root)?;
    // @step And the linked feature's coverage maps "rust/tests/other.rs"
    // @step And "rust/tests/other.rs" does not mention "STORY-1" and was last modified before STORY-1 entered testing
    mapped_old_test_file(root, "rust/tests/other.rs", "// another card's test\n")?;
    // @step When the user moves "STORY-1" to implementing
    let result = move_to_implementing(root);
    // @step Then the move succeeds
    if !result.success {
        return Err(format!(
            "an unrelated mapped test must not block: {:?}",
            result.error
        )
        .into());
    }
    Ok(())
}
