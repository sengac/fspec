//! Feature: spec/features/a-panic-in-one-session-never-stops-other-sessions-persistence.feature
//!
//! BUG-610: the panic that poisoned the daemon's session store was never
//! logged; the panic hooks only restored the terminal and chained to the
//! default hook, which printed to a terminal nobody kept. The panic logging
//! hook writes the panic to the process log first. Everything here is
//! deterministic: the panic is raised on a named worker thread whose
//! tracing output goes to an in-memory writer.

use codelet_common::panic_log;
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    fn text(&self) -> String {
        self.0
            .lock()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|_| "<captured log lock poisoned>".into())
    }
}

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("captured log lock poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for Captured {
    type Writer = Captured;

    fn make_writer(&'writer self) -> Self::Writer {
        self.clone()
    }
}

static PANIC_LINE: AtomicU32 = AtomicU32::new(0);

#[test]
fn a_panic_is_logged_with_its_thread_location_and_session_before_the_earlier_hook_runs(
) -> TestResult {
    // @step Given the panic logging hook is installed over an earlier panic hook
    let log = Captured::default();
    let seen_by_earlier_hook = Arc::new(Mutex::new(None::<String>));
    let earlier_calls = Arc::new(AtomicUsize::new(0));
    {
        let log = log.clone();
        let seen = seen_by_earlier_hook.clone();
        let calls = earlier_calls.clone();
        std::panic::set_hook(Box::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut seen) = seen.lock() {
                *seen = Some(log.text());
            }
        }));
    }
    std::panic::set_hook(panic_log::wrap(std::panic::take_hook(), || {}));
    std::panic::set_hook(panic_log::wrap(std::panic::take_hook(), || {}));

    // @step When code running for session A panics on thread "bug610-worker"
    let session = Uuid::new_v4();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .finish();
    let worker = std::thread::Builder::new()
        .name("bug610-worker".into())
        .spawn(move || {
            tracing::subscriber::with_default(subscriber, || {
                // The hook under test runs only for a real panic, not for
                // resume_unwind, so this test raises one on purpose.
                #[allow(clippy::panic)]
                panic_log::in_session_sync(session, || {
                    PANIC_LINE.store(line!() + 1, Ordering::SeqCst);
                    std::panic::panic_any("injected BUG-610 panic")
                })
            })
        })?;
    let joined = worker.join();
    let _ = std::panic::take_hook();
    assert!(joined.is_err(), "the worker must have panicked");

    // @step Then the log holds one ERROR line with the panic message, the thread name, the panic location and session A's id
    let text = log.text();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("injected BUG-610 panic"))
        .collect();
    let line = lines
        .first()
        .ok_or_else(|| format!("panic not logged: {text}"))?;
    let location = format!("{}:{}", file!(), PANIC_LINE.load(Ordering::SeqCst));
    assert!(line.contains("ERROR"), "{line}");
    assert!(line.contains("thread=bug610-worker"), "{line}");
    assert!(line.contains(&location), "{location} not in {line}");
    assert!(line.contains(&format!("session_id={session}")), "{line}");

    // @step And the line was written before the earlier hook ran
    assert_eq!(earlier_calls.load(Ordering::SeqCst), 1);
    let seen = seen_by_earlier_hook
        .lock()
        .map_err(|_| "seen lock poisoned")?
        .clone()
        .ok_or("the earlier hook did not run")?;
    assert!(seen.contains("injected BUG-610 panic"), "{seen}");

    // @step And a second chained logging hook does not log the panic again
    assert_eq!(lines.len(), 1, "{text}");
    Ok(())
}
