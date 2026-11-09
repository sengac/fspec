//! REFAC-025: fspec-tui test MockBackend split into small modules.
//!
//! Feature: spec/features/tui-test-mock-backend-split.feature

mod common;

use codelet_fspec_tui::FspecBackend;
use codelet_rpc_types::WorkUnitInfo;
use codelet_test_helpers::TestResult;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn every_fspec_tui_test_helper_file_stays_under_300_lines() -> TestResult {
    // @step Given rust/fspec-tui/tests/common and the reconnect test files
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    rust_files(&tests.join("common"), &mut files)?;
    files.push(tests.join("auto_reconnect_slice2_rpc011.rs"));
    files.push(tests.join("reconnect_resubscribe_rpc415.rs"));

    // @step When the line count of every Rust file is measured
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(
        oversized.is_empty(),
        "oversized test helpers: {oversized:?}"
    );
    Ok(())
}

#[tokio::test]
async fn the_mock_backend_behaves_as_before() -> TestResult {
    // @step Given a MockBackend seeded with one work unit
    let mock = common::MockBackend::new();
    let unit = WorkUnitInfo {
        id: "MOCK-1".into(),
        title: "t".into(),
        work_type: "story".into(),
        status: "backlog".into(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    };
    mock.seed_work_units(vec![unit.clone()]);

    // @step When it is used as an FspecBackend
    let backend: &dyn FspecBackend = &mock;
    let units = backend.list_work_units().await?;

    // @step Then list_work_units returns the seeded unit and records the call
    assert_eq!(units, vec![unit]);
    assert_eq!(mock.list_work_units_calls(), 1);
    Ok(())
}
