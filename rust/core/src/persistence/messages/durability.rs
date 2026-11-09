use std::cell::RefCell;
use std::io;
use std::path::Path;

#[derive(Default)]
struct Observation {
    failure: Option<&'static str>,
    /// BUG-625: the boundary at which `perform` panics instead of failing.
    panic: Option<&'static str>,
    completed: Vec<&'static str>,
}

thread_local! {
    static OBSERVATION: RefCell<Option<Observation>> = const { RefCell::new(None) };
}

#[doc(hidden)]
pub fn observe<T>(
    failure: Option<&'static str>,
    operation: impl FnOnce() -> T,
) -> (T, Vec<&'static str>) {
    run_observed(
        Observation {
            failure,
            ..Observation::default()
        },
        operation,
    )
}

/// BUG-625: run `operation` on this thread with a real panic raised at
/// `boundary`, so tests can prove a panic inside persistence work is
/// contained. The observation is restored when the panic unwinds.
#[doc(hidden)]
pub fn panic_at<T>(boundary: &'static str, operation: impl FnOnce() -> T) -> T {
    run_observed(
        Observation {
            panic: Some(boundary),
            ..Observation::default()
        },
        operation,
    )
    .0
}

fn run_observed<T>(
    observation: Observation,
    operation: impl FnOnce() -> T,
) -> (T, Vec<&'static str>) {
    struct Restore(Option<Observation>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OBSERVATION.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(OBSERVATION.with(|slot| slot.replace(Some(observation))));
    let result = operation();
    let completed = OBSERVATION.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .map(|observation| std::mem::take(&mut observation.completed))
            .unwrap_or_default()
    });
    (result, completed)
}

pub(crate) fn perform<T>(
    boundary: &'static str,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let (failure, panic) = OBSERVATION.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or((None, None), |value| (value.failure, value.panic))
    });
    if panic == Some(boundary) {
        // A real panic (not resume_unwind), so the panic hooks run. Reached
        // only inside `panic_at`, which tests call to inject it.
        #[allow(clippy::panic)]
        std::panic::panic_any(format!("injected panic at {boundary}"));
    }
    if failure == Some(boundary) {
        return Err(io::Error::other(format!("injected failure at {boundary}")));
    }
    let result = operation()?;
    OBSERVATION.with(|slot| {
        if let Some(observation) = slot.borrow_mut().as_mut() {
            observation.completed.push(boundary);
        }
    });
    Ok(result)
}

pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durable directory synchronization is unsupported on this platform",
        ))
    }
}

pub(crate) fn sync_parent(path: &Path) -> io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("durable file has no parent directory"))?;
    sync_directory(directory)?;
    if let Some(parent) = directory.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}
