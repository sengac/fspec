// Feature: spec/features/the-spec-lock-is-reentrant-within-a-thread.feature
//
// BUG-477: the spec lock is reentrant for the thread that holds it and
// exclusive for every other process. The probe child (this test binary
// re-executed with BUG477_PROBE_DIR set) tries the raw fs2 protocol once
// without waiting and reports "free" or "busy" on stdout.

use std::path::Path;
use std::process::Command;

use codelet_common::spec_lock::{SpecLock, SPEC_LOCK_FILE_NAME};
use fs2::FileExt;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Child-process entry point. A no-op unless BUG477_PROBE_DIR is set.
#[test]
fn bug477_probe_child() -> TestResult {
    let Ok(dir) = std::env::var("BUG477_PROBE_DIR") else {
        return Ok(());
    };
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(Path::new(&dir).join(SPEC_LOCK_FILE_NAME))?;
    let state = match file.try_lock_exclusive() {
        Ok(()) => {
            FileExt::unlock(&file)?;
            "free"
        }
        Err(_) => "busy",
    };
    println!("BUG477-PROBE={state}");
    Ok(())
}

fn probe(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", "bug477_probe_child", "--nocapture", "--quiet"])
        .env("BUG477_PROBE_DIR", dir)
        .output()?;
    assert!(output.status.success(), "probe child failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let state = stdout
        .lines()
        .find_map(|l| l.strip_prefix("BUG477-PROBE="))
        .ok_or("probe child printed no state")?;
    Ok(state.to_string())
}

#[test]
fn the_lock_is_reentrant_within_a_thread_and_exclusive_across_processes() -> TestResult {
    let project = TempDir::new()?;
    let spec = project.path().join("spec");
    std::fs::create_dir_all(&spec)?;

    // @step Given a thread that holds the spec lock
    let outer = SpecLock::acquire(&spec)?;

    // @step When the same thread acquires the spec lock again
    let inner = SpecLock::acquire(&spec)?;

    // @step Then it gets the lock without waiting
    // TEST-042: no time bound. A non-reentrant acquire would wait on the
    // file lock this thread already holds until the lock timeout and
    // return an error, so getting the guard at all is the proof.

    // @step And another process cannot take the spec lock until the outer guard is dropped
    assert_eq!(probe(&spec)?, "busy");
    drop(inner);
    assert_eq!(
        probe(&spec)?,
        "busy",
        "dropping the inner guard keeps the lock"
    );
    drop(outer);
    assert_eq!(
        probe(&spec)?,
        "free",
        "dropping the outer guard releases it"
    );
    Ok(())
}
