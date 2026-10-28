//! Feature: spec/features/history-read-integrity.feature

use codelet_common::read_work::{self, ReadCancellation};
use codelet_core::persistence;
use std::{
    collections::HashSet,
    io::{self, BufRead, Cursor, Read},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn scenario_cancelled_cold_discovery_never_hides_healthy_sessions() -> TestResult {
    // @step Given healthy session manifests awaiting their first history discovery
    let directory = codelet_test_helpers::setup_temp_directory()?;
    persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().into())?;
    let first = persistence::create_session("first healthy", directory.path())?;
    let second = persistence::create_session("last healthy", directory.path())?;
    let expected = HashSet::from([first.id, second.id]);
    let originals = [first.id, second.id]
        .into_iter()
        .map(|id| {
            let path = directory.path().join("sessions").join(format!("{id}.json"));
            std::fs::read(&path).map(|bytes| (path, bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;
    persistence::reset_session_store_for_tests();
    let cancellation = ReadCancellation::default();
    let token = cancellation.clone();
    let candidates = Arc::new(AtomicUsize::new(0));
    let visited = candidates.clone();
    // @step When cancellation interrupts the last manifest read
    let (result, _) = cancellation.run(|| {
        read_work::observe(
            Arc::new(move |boundary| {
                if boundary == "manifest.candidate" {
                    visited.fetch_add(1, Ordering::AcqRel);
                }
                if boundary == "manifest.bytes" && visited.load(Ordering::Acquire) == 2 {
                    token.cancel();
                }
            }),
            persistence::list_all_sessions,
        )
    });
    assert!(result.is_err());
    assert_eq!(candidates.load(Ordering::Acquire), 2);
    // @step Then a subsequent listing without resetting storage retains every original session
    let listed = persistence::list_all_sessions()?;
    assert_eq!(
        listed
            .iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>(),
        expected
    );
    // @step And healthy files remain unchanged without persistent unreadable diagnostics
    for (path, bytes) in originals {
        assert_eq!(std::fs::read(path)?, bytes);
    }
    assert!(persistence::session_manifest_issues()?.is_empty());
    let broken = directory
        .path()
        .join("sessions")
        .join(format!("{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&broken, "{")?;
    persistence::reset_session_store_for_tests();
    assert_eq!(
        persistence::list_all_sessions()?
            .iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>(),
        expected
    );
    let issues = persistence::session_manifest_issues()?;
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, broken);
    assert_eq!(std::fs::read(broken)?, b"{");
    Ok(())
}

struct InterruptedReader {
    bytes: Cursor<Vec<u8>>,
    pending_interrupt: bool,
    cancellation: Option<ReadCancellation>,
    attempts: usize,
}

impl InterruptedReader {
    fn new(cancellation: Option<ReadCancellation>) -> Self {
        Self {
            bytes: Cursor::new(b"complete line\nremaining bytes".to_vec()),
            pending_interrupt: true,
            cancellation,
            attempts: 0,
        }
    }

    fn attempt(&mut self) -> io::Result<()> {
        self.attempts += 1;
        if std::mem::take(&mut self.pending_interrupt) {
            if let Some(cancellation) = &self.cancellation {
                cancellation.cancel();
            }
            Err(io::Error::from(io::ErrorKind::Interrupted))
        } else {
            Ok(())
        }
    }
}

impl Read for InterruptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.attempt()?;
        self.bytes.read(buffer)
    }
}

impl BufRead for InterruptedReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.attempt()?;
        self.bytes.fill_buf()
    }
    fn consume(&mut self, count: usize) {
        self.bytes.consume(count);
    }
}

#[test]
fn scenario_transient_operating_system_interruptions_preserve_successful_reads() -> TestResult {
    // @step Given readers that receive a transient operating system interruption
    let mut complete = InterruptedReader::new(None);
    let mut delimited = InterruptedReader::new(None);
    let mut bytes = Vec::new();
    let mut line = Vec::new();
    // @step When complete and delimiter-bounded history reads resume
    let complete_result = read_work::read_to_end(&mut complete, &mut bytes, "fixture.bytes");
    let delimited_result = read_work::read_until(&mut delimited, b'\n', &mut line);
    assert!(
        complete_result.is_ok() && delimited_result.is_ok(),
        "complete={complete_result:?}, delimited={delimited_result:?}"
    );
    complete_result?;
    let count = delimited_result?;
    // @step Then both return their complete original bytes
    assert_eq!(bytes, b"complete line\nremaining bytes");
    assert_eq!(line, b"complete line\n");
    assert_eq!(count, line.len());
    let mut remaining = Vec::new();
    delimited.read_to_end(&mut remaining)?;
    assert_eq!(remaining, b"remaining bytes");
    // @step And actual invocation cancellation still stops retries at the next checkpoint
    for delimiter in [false, true] {
        let token = ReadCancellation::default();
        let mut reader = InterruptedReader::new(Some(token.clone()));
        let mut output = Vec::new();
        let result = token.run(|| {
            if delimiter {
                read_work::read_until(&mut reader, b'\n', &mut output).map(|_| ())
            } else {
                read_work::read_to_end(&mut reader, &mut output, "fixture.bytes")
            }
        });
        assert_eq!(
            result.expect_err("cancelled invocation must stop").kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            reader.attempts, 1,
            "checkpoint cancellation was retried as I/O"
        );
        assert!(output.is_empty());
    }
    Ok(())
}
