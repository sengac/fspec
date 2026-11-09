//! BUG-406 — test binaries share one live data directory.
//!
//! Feature: spec/features/test-shared-data-directory.feature

use codelet_test_helpers::shared_temp_directory;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_shared_temp_directory_is_the_same_live_directory_for_every_caller() -> TestResult {
    // @step Given a test binary
    // @step When two callers ask for the shared temp directory
    let first = shared_temp_directory()?;
    let second = std::thread::spawn(|| shared_temp_directory().map(std::path::Path::to_path_buf))
        .join()
        .map_err(|_| "the second caller panicked")??;
    // @step Then both get the same directory and it exists
    assert_eq!(first, second.as_path());
    assert!(first.is_dir(), "{first:?}");
    Ok(())
}

#[test]
fn no_test_drops_the_set_data_directory_result() -> TestResult {
    // @step Given the workspace test sources
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut stack = vec![root];
    let mut offenders = Vec::new();
    // @step When they are scanned for set_data_directory calls
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                return Err(format!("non-UTF-8 path in the workspace: {}", path.display()).into());
            };
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    stack.push(path);
                }
                continue;
            }
            if !name.ends_with(".rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            for line in source.lines() {
                let code = line.trim_start();
                if code.contains("set_data_directory(")
                    && (code.starts_with("let _ =") || code.contains(").ok()"))
                {
                    offenders.push(format!("{}: {}", path.display(), code));
                }
            }
        }
    }
    // @step Then none discards the result with let _ =
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}
