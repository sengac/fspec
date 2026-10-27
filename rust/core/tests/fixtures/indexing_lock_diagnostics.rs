//! Feature: spec/features/indexing-lock-diagnostics.feature
use super::support::{Fixture, TestResult};
use codelet_core::persistence::messages::MessageStore;
use serial_test::serial;
use std::fs::OpenOptions;
use std::sync::{mpsc, Arc, Barrier};
use std::time::Duration;

#[test]
#[serial]
fn shared_file_lease_blocks_unrelated_message_reads_but_not_another_root() -> TestResult {
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
    barrier.wait();
    assert!(independent.get(second.ids[0]).is_some());
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    drop(lease);
    for _ in 0..2 {
        assert!(receiver.recv_timeout(Duration::from_secs(3))?);
    }
    for worker in workers {
        worker.join().map_err(|_| "reader panicked")?;
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn synchronous_locked_get_prevents_unrelated_executor_task_progress() -> TestResult {
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
        let entered = entered_rx.recv_timeout(Duration::from_secs(3));
        let (progress_tx, progress_rx) = mpsc::channel();
        let heartbeat = handle.spawn(async move {
            let _ = progress_tx.send(());
        });
        let pending = progress_rx.recv_timeout(Duration::from_millis(100));
        drop(lease); // Always release before returning, including failed controls.
        (entered, pending, heartbeat)
    });
    let id = fixture.ids[0];
    let blocked = tokio::spawn(async move {
        let _ = entered_tx.send(());
        reader.get(id).is_some()
    });
    assert!(blocked.await?);
    let (entered, pending, heartbeat) = controller.join().map_err(|_| "controller panicked")?;
    entered?;
    assert!(
        matches!(pending, Err(mpsc::RecvTimeoutError::Timeout)),
        "unrelated task unexpectedly progressed while the only worker was blocked"
    );
    heartbeat.await?;
    let control = tokio::spawn(async { true });
    assert!(control.await?, "unlocked executor control did not progress");
    Ok(())
}
