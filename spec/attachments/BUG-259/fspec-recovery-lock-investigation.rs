use codelet_agent_loop::persistence_retry::PersistenceSpool;
use codelet_core::persistence::{create_session, session_lifecycle::RecoveryLease};
use codelet_test_helpers::setup_temp_directory;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn temporary_recovery_lease_contention_is_not_a_fatal_open() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    let session = create_session("contention probe", directory.path())?;
    let held = RecoveryLease::acquire(directory.path(), session.id)?;
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        drop(held);
    });
    let started = Instant::now();
    let result = PersistenceSpool::open(session.id);
    let elapsed = started.elapsed();
    release.join().map_err(|_| "lease release thread failed")?;
    assert!(PersistenceSpool::open(session.id).is_ok());
    assert!(
        result.is_ok(),
        "transient contention failed after {elapsed:?}: {:?}",
        result.err()
    );
    Ok(())
}

#[test]
fn recovery_leases_without_process_spawning_do_not_contend() -> TestResult {
    let directory = setup_temp_directory()?;
    let session = codelet_core::persistence::manifest::SessionManifest::new(
        "no spawn control",
        directory.path().into(),
    );
    for _ in 0..50_000 {
        drop(RecoveryLease::acquire(directory.path(), session.id)?);
    }
    Ok(())
}

#[test]
fn explicit_unlock_before_close_prevents_spawn_inheritance_contention() -> TestResult {
    use std::fs::OpenOptions;
    use std::os::unix::process::CommandExt;
    let directory = setup_temp_directory()?;
    let path = directory.path().join("explicit-unlock.lock");
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let locker = std::thread::spawn(move || -> Result<usize, String> {
        let mut failures = 0;
        while !worker_stop.load(Ordering::Acquire) {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            if file.try_lock().is_err() {
                failures += 1;
            } else {
                file.unlock().map_err(|error| error.to_string())?;
            }
        }
        Ok(failures)
    });
    for _ in 0..500 {
        let status = Command::new("/bin/sh")
            .args(["-c", "true"])
            .process_group(0)
            .status()?;
        assert!(status.success());
    }
    stop.store(true, Ordering::Release);
    let failures = locker.join().map_err(|_| "locker thread failed")??;
    assert_eq!(failures, 0);
    Ok(())
}

#[test]
fn unrelated_process_spawning_does_not_contend_with_recovery_leases() -> TestResult {
    use std::os::unix::process::CommandExt;
    let directory = setup_temp_directory()?;
    let manifest = codelet_core::persistence::manifest::SessionManifest::new(
        "spawn probe",
        directory.path().into(),
    );
    let session_id = manifest.id;
    let stop = Arc::new(AtomicBool::new(false));
    let iterations = Arc::new(AtomicUsize::new(0));
    let root = directory.path().to_path_buf();
    let worker_stop = stop.clone();
    let worker_iterations = iterations.clone();
    let locker = std::thread::spawn(move || {
        let mut failures = Vec::new();
        while !worker_stop.load(Ordering::Acquire) {
            if let Err(error) = RecoveryLease::acquire(&root, session_id) {
                failures.push(error.to_string());
            }
            worker_iterations.fetch_add(1, Ordering::Relaxed);
        }
        failures
    });
    let mut spawn_error = None;
    for _ in 0..500 {
        match Command::new("/bin/sh")
            .args(["-c", "true"])
            .process_group(0)
            .status()
        {
            Ok(status) if status.success() => {}
            result => {
                spawn_error = Some(format!("spawn failed: {result:?}"));
                break;
            }
        }
    }
    stop.store(true, Ordering::Release);
    let failures = locker.join().map_err(|_| "locker thread failed")?;
    assert!(spawn_error.is_none(), "{spawn_error:?}");
    assert!(
        failures.is_empty(),
        "{} failures in {} uncontested lease acquisitions; first: {:?}",
        failures.len(),
        iterations.load(Ordering::Relaxed),
        failures.first()
    );
    Ok(())
}
