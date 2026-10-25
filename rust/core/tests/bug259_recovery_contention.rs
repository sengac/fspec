#![doc = "Feature: spec/features/bounded-recovery-lock-contention.feature"]

use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_core::persistence::{
    create_session, reset_stores_for_tests,
    session_lifecycle::RecoveryLease,
    transcript::{JournalEvent, JournalIdentity, JournalRecord, JournalStore},
};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn short_recovery_contention_preserves_authentic_pending_output() -> TestResult {
    // @step Given a durable session with authentic pending output and a short lived recovery lock holder
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let session = create_session("short contention", directory.path())?;
    let spool = PersistenceSpool::open(session.id)?;
    let record = JournalRecord::new(
        JournalIdentity {
            session_id: session.id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Text {
            text: "authentic output α".into(),
        },
    );
    spool.enqueue(&RecoveryRecord {
        version: 1,
        id: record.id,
        role: "transcript".into(),
        content: String::new(),
        metadata: HashMap::from([("_transcript".into(), serde_json::to_value(&record)?)]),
    })?;
    let held = RecoveryLease::acquire(directory.path(), session.id)?;
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        drop(held);
    });
    // @step When persistence opens while the holder releases the lock
    let result = PersistenceSpool::open(session.id);
    release.join().map_err(|_| "lease release thread failed")?;
    // @step Then the original output is recovered exactly once without interruption
    let reopened = result?;
    assert_eq!(reopened.pending_count()?, 1);
    reopened.retry()?;
    reopened.retry()?;
    assert_eq!(reopened.pending_count()?, 0);
    assert_eq!(JournalStore::configured(session.id)?.read()?, vec![record]);
    Ok(())
}

#[test]
fn persistent_contention_is_bounded_and_isolated() -> TestResult {
    // @step Given a recovery lock held beyond the contention deadline
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let held = RecoveryLease::acquire(directory.path(), session_id)?;
    let root = directory.path().to_path_buf();
    let (sender, receiver) = mpsc::channel();
    let (started_sender, started_receiver) = mpsc::channel();
    // @step When the locked session and an independent session acquire recovery ownership
    let worker = std::thread::spawn(move || {
        let _ = started_sender.send(());
        let started = Instant::now();
        let result = RecoveryLease::acquire(&root, session_id).map(drop);
        let _ = sender.send((result, started.elapsed()));
    });
    started_receiver.recv()?;
    let independent = RecoveryLease::acquire(directory.path(), Uuid::new_v4());
    let outcome = receiver.recv_timeout(Duration::from_secs(2));
    drop(held);
    worker.join().map_err(|_| "contention worker failed")?;
    // @step Then the independent session progresses and the locked session returns WouldBlock within a finite deadline
    independent?;
    let (result, elapsed) = outcome?;
    let error = result
        .err()
        .ok_or("persistent contention acquired ownership")?;
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    assert!(elapsed >= Duration::from_millis(200), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    drop(RecoveryLease::acquire(directory.path(), session_id)?);
    Ok(())
}

#[test]
#[cfg(unix)]
fn shell_spawning_does_not_extend_released_recovery_ownership() -> TestResult {
    use std::os::unix::process::CommandExt;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    // @step Given a session repeatedly releasing recovery ownership
    let directory = setup_temp_directory()?;
    let root = directory.path().to_path_buf();
    let session_id = Uuid::new_v4();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || {
        let mut failures = Vec::new();
        let mut acquisitions = 0;
        while !worker_stop.load(Ordering::Acquire) {
            if let Err(error) = RecoveryLease::acquire(&root, session_id) {
                failures.push(error.to_string());
            }
            acquisitions += 1;
        }
        (acquisitions, failures)
    });
    // @step When unrelated shells spawn concurrently
    let spawning = (0..500).try_for_each(|_| {
        std::process::Command::new("/bin/sh")
            .args(["-c", "true"])
            .process_group(0)
            .status()
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other("shell failed"))
                }
            })
    });
    stop.store(true, Ordering::Release);
    let (acquisitions, failures) = worker.join().map_err(|_| "lease worker failed")?;
    spawning?;
    // @step Then every acquisition succeeds without false contention
    assert!(acquisitions > 0);
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn non_contention_filesystem_failures_propagate_immediately() -> TestResult {
    // @step Given an invalid recovery lock path
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let parent = directory.path().join("persistence-recovery");
    std::fs::create_dir_all(parent.join(format!("{session_id}.lock")))?;
    // @step When recovery ownership is acquired
    let started = Instant::now();
    let result = RecoveryLease::acquire(directory.path(), session_id);
    // @step Then the filesystem error is returned without contention retries
    let error = result
        .err()
        .ok_or("directory unexpectedly opened as a lock")?;
    assert_ne!(error.kind(), std::io::ErrorKind::WouldBlock);
    assert!(started.elapsed() < Duration::from_millis(200));
    Ok(())
}
