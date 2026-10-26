#![doc = "Feature: spec/features/streaming-persistence-speed-simulations.feature"]

use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::{JournalEvent, JournalStore};
use codelet_rpc_types::StreamChunk;
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[path = "support/streaming_speed.rs"]
mod fixture;
use fixture::TestResult;

#[path = "support/streaming_records.rs"]
mod journal_fixture;

fn deltas(count: usize, bytes: usize) -> Vec<StreamEvent> {
    (0..count)
        .map(|index| {
            let content = format!("{index:08} {} ", "α".repeat(bytes / 2));
            if index % 2 == 0 {
                StreamEvent::Text(content)
            } else {
                StreamEvent::Thinking(content)
            }
        })
        .collect()
}

fn report(label: &str, elapsed: Duration, latencies: &mut [Duration]) {
    latencies.sort();
    let count = latencies.len();
    let percentile = |percent: usize| latencies[(count - 1) * percent / 100].as_secs_f64() * 1000.0;
    println!(
        "{label}: deltas={count} elapsed_ms={:.3} deltas_per_sec={:.1} p50_ms={:.3} p95_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0,
        count as f64 / elapsed.as_secs_f64(),
        percentile(50),
        percentile(95)
    );
}

#[tokio::test]
#[serial_test::serial]
async fn fixture_streams_separate_publication_cost_from_provider_latency() -> TestResult {
    // @step Given prebuilt text and thinking deltas with no network or generation delay
    for bytes in [16, 1024] {
        let (directory, session, output) = fixture::background()?;
        let events = deltas(256, bytes);
        let mut baseline_latency = Vec::new();
        // @step When identical streams use direct publication and durable background output
        let baseline_started = Instant::now();
        for event in &events {
            let started = Instant::now();
            let chunk = match event {
                StreamEvent::Text(text) => StreamChunk::text(text.clone()),
                StreamEvent::Thinking(thinking) => StreamChunk::thinking(thinking.clone()),
                _ => return Err("unexpected fixture event".into()),
            };
            session.handle_output(chunk);
            baseline_latency.push(started.elapsed());
        }
        let baseline_elapsed = baseline_started.elapsed();
        let mut durable_latency = Vec::new();
        let durable_started = Instant::now();
        let (result, operations) = observe(None, || -> TestResult {
            for event in &events {
                let started = Instant::now();
                match event {
                    StreamEvent::Text(text) => output.emit(StreamEvent::Text(text.clone())),
                    StreamEvent::Thinking(thinking) => {
                        output.emit(StreamEvent::Thinking(thinking.clone()))
                    }
                    _ => return Err("unexpected fixture event".into()),
                }
                durable_latency.push(started.elapsed());
            }
            Ok(())
        });
        result?;
        let durable_elapsed = durable_started.elapsed();
        // @step Then the report includes throughput latency and per-delta durability work
        report(
            &format!("direct-{bytes}"),
            baseline_elapsed,
            &mut baseline_latency,
        );
        report(
            &format!("durable-{bytes}"),
            durable_elapsed,
            &mut durable_latency,
        );
        let count = |label| {
            operations
                .iter()
                .filter(|operation| **operation == label)
                .count()
        };
        assert_eq!(count("transcript.record.sync"), events.len());
        assert_eq!(count("transcript.directory.sync"), events.len());
        assert_eq!(count("transcript.root.sync"), events.len());
        assert_eq!(count("transcript.history.read"), 0);
        println!("observed_journal_boundaries={} historical_reads={} recovery_barriers_not_in_observer=true", events.len() * 3, count("transcript.history.read"));
        // @step And cold recovery contains every durably published delta exactly once
        let recovered = JournalStore::new(directory.path(), session.id).read()?;
        assert_eq!(recovered.len(), events.len());
        assert_eq!(
            recovered
                .iter()
                .map(|record| record.id)
                .collect::<HashSet<_>>()
                .len(),
            events.len()
        );
        for (record, expected) in recovered.iter().zip(&events) {
            match (&record.event, expected) {
                (JournalEvent::Text { text: actual }, StreamEvent::Text(expected)) => {
                    assert_eq!(actual, expected)
                }
                (JournalEvent::Thinking { thinking: actual }, StreamEvent::Thinking(expected)) => {
                    assert_eq!(actual, expected)
                }
                _ => return Err("cold recovery changed the fixture channel".into()),
            }
        }
    }
    Ok(())
}

#[test]
fn cold_admission_reproduces_historical_replay_amplification() -> TestResult {
    // @step Given fixed-size journal fixtures in growing batches
    for count in [64, 128, 256] {
        let records = journal_fixture::records(count);
        let session_id = records[0].identity.session_id;
        for cold in [false, true] {
            let directory = setup_temp_directory()?;
            let store = JournalStore::new(directory.path(), session_id);
            // @step When identical records use retained writers and cold writers per delta
            let started = Instant::now();
            let (result, operations) = observe(None, || -> TestResult {
                for record in &records {
                    if cold {
                        JournalStore::new(directory.path(), session_id).append(record)?;
                    } else {
                        store.append(record)?;
                    }
                }
                Ok(())
            });
            result?;
            let reads = operations
                .iter()
                .filter(|operation| **operation == "transcript.history.read")
                .count();
            println!(
                "admission cold={cold} deltas={count} elapsed_ms={:.3} historical_reads={reads}",
                started.elapsed().as_secs_f64() * 1000.0
            );
            // @step Then cold historical reads follow the triangular growth formula
            // @step And retained writers avoid repeated historical reads
            assert_eq!(reads, if cold { count * (count - 1) / 2 } else { 0 });
            assert_eq!(store.read()?, records);
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[serial_test::serial]
async fn a_stalled_admission_prevents_cooperative_executor_progress() -> TestResult {
    // @step Given a background stream on a single-thread executor and a locked journal writer
    let (directory, session, output) = fixture::background()?;
    let store = JournalStore::new(directory.path(), session.id);
    std::fs::create_dir_all(store.directory())?;
    let lease = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(store.directory().join("writer.lock"))?;
    lease.lock()?;
    let progressed = Arc::new(AtomicBool::new(false));
    let sibling_progressed = progressed.clone();
    let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
    let sibling = tokio::spawn(async move {
        let _ = started_sender.send(());
        tokio::task::yield_now().await;
        sibling_progressed.store(true, Ordering::SeqCst);
    });
    started_receiver.await?;
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let mut subscriber = session.subscribe_to_stream();
    let mut published = session.subscribe_to_stream();
    let releaser = std::thread::spawn(move || -> Result<bool, std::io::Error> {
        release_receiver.recv().map_err(std::io::Error::other)?;
        std::thread::sleep(Duration::from_millis(200));
        let premature_publication = subscriber.try_recv().is_ok();
        let starved = !progressed.load(Ordering::SeqCst);
        lease.unlock()?;
        Ok(starved && !premature_publication)
    });
    let started = Instant::now();
    release_sender.send(())?;
    // @step When a ready sibling task waits while a fixture delta is emitted
    output.emit(StreamEvent::Text("fixture under writer contention".into()));
    let blocked_elapsed = started.elapsed();
    sibling.await?;
    // @step Then the sibling cannot progress before an external thread releases the writer
    assert!(releaser.join().map_err(|_| "release thread failed")??);
    println!(
        "single_thread_executor blocked_ms={:.3} sibling_progress_before_unlock=false",
        blocked_elapsed.as_secs_f64() * 1000.0
    );
    // @step And publication happens only after durable admission
    assert!(matches!(published.try_recv()?, StreamChunk::Text { .. }));
    assert_eq!(store.read()?.len(), 1);
    Ok(())
}
