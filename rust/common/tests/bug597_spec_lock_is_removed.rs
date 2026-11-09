//! BUG-597: the spec write lock file exists only while a writer holds the
//! lock; releasing removes it, and writers stay mutually exclusive.
//!
//! Feature: spec/features/the-spec-write-lock-is-removed-when-the-write-finishes.feature
//!
//! The concurrent-process scenario re-executes this test binary with
//! BUG597_COUNTER_SPEC set: each child increments the counter file under the
//! spec lock and exits.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use codelet_common::spec_lock::{SpecLock, SPEC_LOCK_FILE_NAME, SPEC_LOCK_TIMEOUT_ENV};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CHILD_ENV: &str = "BUG597_COUNTER_SPEC";
const CHILD_TEST: &str = "concurrent_processes_serialize_and_leave_no_lock_file";
const PROCESSES: usize = 8;
const INCREMENTS: usize = 25;

fn spec_dir(root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let spec = root.join("spec");
    fs::create_dir_all(&spec)?;
    Ok(spec.canonicalize()?)
}

/// The lock files in `spec`.
fn lock_files(spec: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(spec)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".lock") {
            found.push(name);
        }
    }
    Ok(found)
}

#[test]
#[serial_test::serial]
fn releasing_the_lock_removes_the_lock_file() -> TestResult {
    // @step Given a spec directory with no lock file
    let root = tempfile::tempdir()?;
    let spec = spec_dir(root.path())?;
    let lock = spec.join(SPEC_LOCK_FILE_NAME);
    assert!(!lock.exists());

    // @step When a writer takes the spec lock, takes it again on the same thread, and releases both
    let outer = SpecLock::acquire(&spec)?;
    // @step Then spec/.fspec-write.lock exists while the writer holds the lock
    assert!(lock.exists());
    let inner = SpecLock::acquire(&spec)?;
    drop(inner);
    // @step And it still exists after the inner release
    assert!(lock.exists());
    drop(outer);
    // @step And it is gone after the outer release
    assert!(!lock.exists());
    assert!(lock_files(&spec)?.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn a_lock_file_left_by_a_killed_holder_never_blocks() -> TestResult {
    // @step Given spec/.fspec-write.lock left behind with no process holding it
    let root = tempfile::tempdir()?;
    let spec = spec_dir(root.path())?;
    let lock = spec.join(SPEC_LOCK_FILE_NAME);
    fs::write(&lock, "")?;

    // @step When a writer takes the spec lock with a 100 ms timeout
    std::env::set_var(SPEC_LOCK_TIMEOUT_ENV, "100");
    let acquired = SpecLock::acquire(&spec);
    std::env::remove_var(SPEC_LOCK_TIMEOUT_ENV);

    // @step Then it holds the lock at once
    let guard = acquired?;
    assert!(lock.exists());
    drop(guard);
    // @step And the file is gone after it releases
    assert!(!lock.exists());
    Ok(())
}

#[test]
#[serial_test::serial]
fn a_waiter_whose_locked_file_was_removed_starts_again_on_the_file_at_the_path() -> TestResult {
    // @step Given writer A holds the spec lock
    let root = tempfile::tempdir()?;
    let spec = spec_dir(root.path())?;
    let lock = spec.join(SPEC_LOCK_FILE_NAME);
    let writer_a = SpecLock::acquire(&spec)?;

    // @step And writer B is waiting for it
    let (started, waiting) = mpsc::channel();
    let (held, holding) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let spec_b = spec.clone();
    let lock_b = lock;
    let writer_b = std::thread::spawn(move || -> Result<(), String> {
        started.send(()).map_err(|e| e.to_string())?;
        let guard = SpecLock::acquire(&spec_b).map_err(|e| e.to_string())?;
        // While B holds the lock, its file is the one at the path.
        held.send(lock_b.exists()).map_err(|e| e.to_string())?;
        released.recv().map_err(|e| e.to_string())?;
        drop(guard);
        Ok(())
    });
    waiting.recv()?;

    // @step When A releases the lock
    drop(writer_a);

    // @step Then B holds the lock on the file now at spec/.fspec-write.lock
    assert!(
        holding.recv()?,
        "B holds a lock file that is not at the path"
    );
    release.send(())?;
    writer_b.join().map_err(|_| "writer B panicked")??;
    // @step And no lock file remains after B releases
    assert!(lock_files(&spec)?.is_empty());
    Ok(())
}

/// Child role: increment the counter under the spec lock, INCREMENTS times.
fn increment_as_child(spec: &Path) -> TestResult {
    let counter = spec.join("counter");
    for _ in 0..INCREMENTS {
        let _guard = SpecLock::acquire(spec)?;
        // The holder's lock file is the one at the path while it holds it.
        if !spec.join(SPEC_LOCK_FILE_NAME).exists() {
            return Err("the lock file is missing while this child holds the lock".into());
        }
        let value: usize = fs::read_to_string(&counter)?.trim().parse()?;
        fs::write(&counter, (value + 1).to_string())?;
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn concurrent_processes_serialize_and_leave_no_lock_file() -> TestResult {
    if let Some(spec) = std::env::var_os(CHILD_ENV) {
        return increment_as_child(Path::new(&spec));
    }
    // @step Given eight processes that each increment a counter file 25 times under the spec lock
    let root = tempfile::tempdir()?;
    let spec = spec_dir(root.path())?;
    fs::write(spec.join("counter"), "0")?;

    // @step When they run at the same time
    let children = (0..PROCESSES)
        .map(|_| {
            Command::new(std::env::current_exe()?)
                .args(["--exact", CHILD_TEST, "--nocapture"])
                .env(CHILD_ENV, &spec)
                .spawn()
        })
        .collect::<Result<Vec<_>, _>>()?;
    for mut child in children {
        assert!(child.wait()?.success(), "a child failed");
    }

    // @step Then the counter is 200
    assert_eq!(
        fs::read_to_string(spec.join("counter"))?,
        (PROCESSES * INCREMENTS).to_string()
    );
    // @step And spec/ holds no lock file
    assert!(lock_files(&spec)?.is_empty(), "{:?}", lock_files(&spec)?);
    Ok(())
}
