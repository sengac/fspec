use std::cell::RefCell;
use std::io;
use std::path::Path;

#[derive(Default)]
struct Observation {
    failure: Option<&'static str>,
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
    struct Restore(Option<Observation>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OBSERVATION.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(OBSERVATION.with(|slot| {
        slot.replace(Some(Observation {
            failure,
            completed: Vec::new(),
        }))
    }));
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
    OBSERVATION.with(|slot| {
        if slot.borrow().as_ref().and_then(|value| value.failure) == Some(boundary) {
            return Err(io::Error::other(format!("injected failure at {boundary}")));
        }
        Ok(())
    })?;
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
