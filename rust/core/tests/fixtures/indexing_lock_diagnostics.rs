//! Feature: spec/features/indexing-lock-diagnostics.feature
use super::support::{Fixture, TestResult};
use codelet_core::persistence::messages::MessageStore;
use serial_test::serial;
use std::fs::OpenOptions;
use std::sync::{mpsc, Arc, Barrier};

#[test]
#[serial]
fn shared_file_lease_blocks_unrelated_message_reads_but_not_another_root() -> TestResult {
    // @step Given two readers and an exclusively held fixture sidecar lease
    let first = Fixture::new(2, 64)?;
    let readers = [MessageStore::new()?, MessageStore::new()?];
    let second = Fixture::new(1, 64)?;
    let independent = MessageStore::new()?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(first.directory.path().join("messages.lock"))?;
    lease.lock()?;
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(first.directory.path().join("messages.lock"))?;
    assert!(matches!(
        contender.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    let barrier = Arc::new(Barrier::new(3));
    let (sender, receiver) = mpsc::channel();
    let mut workers = Vec::new();
    for (reader, id) in readers.into_iter().zip(first.ids.iter().copied()) {
        let ready = Arc::clone(&barrier);
        let sender = sender.clone();
        workers.push(std::thread::spawn(move || {
            ready.wait();
            let found = reader.get(id).is_some();
            let _ = sender.send(found);
        }));
    }
    // @step When an independent root is read
    barrier.wait();
    assert!(independent.get(second.ids[0]).is_some());
    // @step Then the independent root progresses before the lease is released
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    // @step And both shared-root readers finish after release
    drop(lease);
    // TEST-042: wait for both readers (no timeout).
    for _ in 0..2 {
        assert!(receiver.recv()?);
    }
    for worker in workers {
        worker.join().map_err(|_| "reader panicked")?;
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn synchronous_locked_get_prevents_unrelated_executor_task_progress() -> TestResult {
    // @step Given an exclusively held synthetic message-store lease
    let fixture = Fixture::new(1, 64)?;
    let reader = MessageStore::new()?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.directory.path().join("messages.lock"))?;
    lease.lock()?;
    let (entered_tx, entered_rx) = mpsc::channel();
    let handle = tokio::runtime::Handle::current();
    let controller = std::thread::spawn(move || {
        let entered = entered_rx.recv();
        let (progress_tx, progress_rx) = mpsc::channel();
        let heartbeat = handle.spawn(async move {
            let _ = progress_tx.send(());
        });
        // TEST-042: no waiting window; while the only worker is blocked in
        // the getter, the heartbeat cannot have run yet.
        let pending = progress_rx.try_recv();
        drop(lease); // Always release before returning, including failed controls.
        (entered, pending, heartbeat)
    });
    // @step When the real synchronous getter runs on one async worker
    let id = fixture.ids[0];
    let blocked = tokio::spawn(async move {
        let _ = entered_tx.send(());
        reader.get(id).is_some()
    });
    assert!(blocked.await?);
    let (entered, pending, heartbeat) = controller.join().map_err(|_| "controller panicked")?;
    entered?;
    // @step Then an unrelated task waits until the external controller releases the lease
    assert!(
        matches!(pending, Err(mpsc::TryRecvError::Empty)),
        "unrelated task unexpectedly progressed while the only worker was blocked"
    );
    heartbeat.await?;
    // @step And the unlocked executor control progresses
    let control = tokio::spawn(async { true });
    assert!(control.await?, "unlocked executor control did not progress");
    Ok(())
}
