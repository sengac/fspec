//! BUG-477: the one cross-process write lock for a project's `spec/` JSON
//! files, plus the atomic file replace every spec write goes through.
//!
//! Feature: spec/features/concurrent-commands-never-corrupt-spec-json-files.feature
//!
//! Protocol:
//!   - The lock is an exclusive advisory lock (fs2: `flock` on Unix,
//!     `LockFileEx` on Windows) on [`SPEC_LOCK_FILE_NAME`] inside the
//!     directory being written (normally `spec/`); the spec files themselves
//!     are replaced by rename and cannot be the lock target. The kernel
//!     releases the lock when its holder exits, so no stale-lock detection is
//!     needed (unlike the TS mkdir lock).
//!   - BUG-597: the lock file exists only while it is held. The last release
//!     removes it while still holding the lock, then unlocks. Acquiring opens
//!     or creates the file, locks it, and checks that the locked file is
//!     still the file at the path (same-file identity: device + inode, or
//!     volume + file index on Windows); when it is not (a releaser removed
//!     it, or a newer one replaced it) the acquirer drops it and starts
//!     again, so two writers never hold locks at once. A file left by a
//!     killed holder is locked at once by the next writer and removed when
//!     it releases.
//!   - Acquisition waits on the OS lock (the kernel queues waiters, so a
//!     busy lock is handed over instead of being raced by sleeping pollers)
//!     for at most [`SPEC_LOCK_TIMEOUT`], or the whole number of
//!     milliseconds in [`SPEC_LOCK_TIMEOUT_ENV`] when that is set, and then
//!     fails with "Failed to acquire file lock on <lock file> after waiting
//!     <n> ms. Check for stuck fspec processes." (`io::ErrorKind::TimedOut`).
//!     LOCK-001's five-second budget starved concurrent writers once each
//!     write fsyncs a large board under build load, so the default is a
//!     minute; a crashed holder never needs it because the kernel drops its
//!     lock.
//!   - A thread that already holds the lock for a directory acquires it
//!     again without waiting (reentrant guard count); every other thread
//!     and process waits.
//!   - [`replace_file_atomically`] writes a uniquely named temp file in the
//!     target's directory, fsyncs it, renames it over the target and fsyncs
//!     the directory. A failed write removes its temp file and leaves the
//!     target untouched.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use fs2::FileExt;

/// Name of the stable lock file inside a locked directory.
pub const SPEC_LOCK_FILE_NAME: &str = ".fspec-write.lock";

/// Longest wait for the lock before giving up, unless
/// [`SPEC_LOCK_TIMEOUT_ENV`] says otherwise.
pub const SPEC_LOCK_TIMEOUT: Duration = Duration::from_secs(60);

/// Environment variable overriding [`SPEC_LOCK_TIMEOUT`] with a whole number
/// of milliseconds. Any other value is an error, never ignored.
pub const SPEC_LOCK_TIMEOUT_ENV: &str = "FSPEC_SPEC_LOCK_TIMEOUT_MS";

fn lock_timeout() -> io::Result<Duration> {
    match std::env::var(SPEC_LOCK_TIMEOUT_ENV) {
        Err(std::env::VarError::NotPresent) => Ok(SPEC_LOCK_TIMEOUT),
        Ok(raw) => raw
            .trim()
            .parse::<u64>()
            .map(Duration::from_millis)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                    "{SPEC_LOCK_TIMEOUT_ENV} must be a whole number of milliseconds, got {raw:?}"
                ),
                )
            }),
        Err(std::env::VarError::NotUnicode(raw)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{SPEC_LOCK_TIMEOUT_ENV} must be a whole number of milliseconds, got {raw:?}"),
        )),
    }
}

struct Holder {
    thread: ThreadId,
    depth: usize,
    file: File,
    /// BUG-597: removed by the last release, while still locked.
    path: PathBuf,
}

fn registry() -> MutexGuard<'static, HashMap<PathBuf, Holder>> {
    static HELD: OnceLock<Mutex<HashMap<PathBuf, Holder>>> = OnceLock::new();
    let mutex = HELD.get_or_init(|| Mutex::new(HashMap::new()));
    // A panic while the registry is locked cannot leave a holder half
    // updated (every critical section is a single insert/update/remove),
    // so a poisoned registry is still consistent.
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Exclusive guard over one directory's spec files; released on drop.
#[derive(Debug)]
#[must_use = "the spec lock is released as soon as the guard is dropped"]
pub struct SpecLock {
    key: PathBuf,
}

impl SpecLock {
    /// Block (within the retry budget) until this thread holds the spec
    /// lock of `dir` (WT-015: of its state root, see `lock_dir_for`).
    pub fn acquire(dir: &Path) -> io::Result<SpecLock> {
        let key = crate::spec_paths::lock_dir_for(dir)?.canonicalize()?;
        let me = thread::current().id();
        if let Some(holder) = registry().get_mut(&key) {
            if holder.thread == me {
                holder.depth += 1;
                return Ok(SpecLock { key });
            }
        }
        let lock_path = key.join(SPEC_LOCK_FILE_NAME);
        let timeout = lock_timeout()?;
        let deadline = Instant::now() + timeout;
        let file = loop {
            let file = match open_lock_file(&lock_path) {
                Ok(file) => file,
                Err(error) if is_pending_removal(&error) => {
                    wait_or_time_out(deadline, timeout, &lock_path)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            let file = lock_within_timeout(file, &lock_path, deadline, timeout)?;
            match is_the_file_at(&file, &lock_path) {
                Ok(true) => break file,
                // BUG-597: a releaser removed this file, or a newer one
                // replaced it: start again on the file at the path.
                Ok(false) => {}
                Err(error) if is_pending_removal(&error) => {}
                Err(error) => return Err(error),
            }
            let _ = FileExt::unlock(&file);
            wait_or_time_out(deadline, timeout, &lock_path)?;
        };
        registry().insert(
            key.clone(),
            Holder {
                thread: me,
                depth: 1,
                file,
                path: lock_path,
            },
        );
        Ok(SpecLock { key })
    }

    /// The canonical directory this guard locks.
    pub fn dir(&self) -> &Path {
        &self.key
    }
}

impl Drop for SpecLock {
    fn drop(&mut self) {
        let released = {
            let mut held = registry();
            match held.get_mut(&self.key) {
                Some(holder) if holder.depth > 1 => {
                    holder.depth -= 1;
                    None
                }
                Some(_) => held.remove(&self.key),
                None => None,
            }
        };
        if let Some(holder) = released {
            // BUG-597: remove the file while still holding the lock, so a
            // waiter that then locks it sees it is gone and starts again.
            if let Err(error) = std::fs::remove_file(&holder.path) {
                tracing::warn!(path = %holder.path.display(), %error, "cannot remove the spec lock file");
            }
            let _ = FileExt::unlock(&holder.file);
        }
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

/// BUG-597: whether `file` (locked) is the file now at `path`.
fn is_the_file_at(file: &File, path: &Path) -> io::Result<bool> {
    let held = same_file::Handle::from_file(file.try_clone()?)?;
    match same_file::Handle::from_path(path) {
        Ok(at_path) => Ok(held == at_path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Windows keeps a removed file that is still open as "delete pending":
/// opening it fails with access denied until the last handle closes. That
/// is the removal in progress, so the acquirer starts again. Unix removes
/// the name at once.
fn is_pending_removal(error: &io::Error) -> bool {
    cfg!(windows) && error.kind() == io::ErrorKind::PermissionDenied
}

/// Before starting the acquisition again: fail with the timeout error once
/// the deadline has passed, otherwise let other threads run first.
fn wait_or_time_out(deadline: Instant, timeout: Duration, lock_path: &Path) -> io::Result<()> {
    if Instant::now() >= deadline {
        return Err(timed_out(lock_path, timeout));
    }
    thread::yield_now();
    Ok(())
}

fn timed_out(lock_path: &Path, timeout: Duration) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "Failed to acquire file lock on {} after waiting {} ms. Check for stuck fspec processes.",
            lock_path.display(),
            timeout.as_millis()
        ),
    )
}

/// Take the exclusive lock on `file`, waiting at most the lock timeout.
///
/// The uncontended case locks inline. Otherwise a helper thread blocks on
/// the OS lock and hands the locked file back; if the deadline passes first
/// the handover fails and the helper releases the lock the moment it gets
/// it, so an abandoned wait never keeps the lock.
fn lock_within_timeout(
    file: File,
    lock_path: &Path,
    deadline: Instant,
    timeout: Duration,
) -> io::Result<File> {
    match file.try_lock_exclusive() {
        Ok(()) => return Ok(file),
        Err(e) if !is_contended(&e) => return Err(e),
        Err(_) => {}
    }
    let (handover, received) = mpsc::sync_channel::<io::Result<File>>(1);
    thread::Builder::new()
        .name("spec-lock-wait".to_string())
        .spawn(move || {
            let locked = file.lock_exclusive().map(|()| file);
            if let Err(
                mpsc::TrySendError::Full(Ok(file)) | mpsc::TrySendError::Disconnected(Ok(file)),
            ) = handover.try_send(locked)
            {
                let _ = FileExt::unlock(&file);
            }
        })?;
    match received.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(locked) => locked,
        Err(_) => Err(timed_out(lock_path, timeout)),
    }
}

fn is_contended(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::WouldBlock
        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
}

/// Run `f` while this thread holds the spec lock of `dir`.
pub fn with_spec_lock<T, E>(dir: &Path, f: impl FnOnce() -> Result<T, E>) -> Result<T, E>
where
    E: From<io::Error>,
{
    let _guard = SpecLock::acquire(dir)?;
    f()
}

/// Replace `path` with `bytes` atomically: write a uniquely named temp
/// file in the same directory, fsync it, rename it over `path`, then fsync
/// the directory. On failure the temp file is removed and `path` keeps its
/// previous content.
pub fn replace_file_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let file_name = path
        .file_name()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no file name", path.display()),
            )
        })?
        .to_string_lossy()
        .to_string();
    let temp = dir.join(format!(".{file_name}.tmp.{}", uuid::Uuid::new_v4()));
    let written = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    sync_dir(&dir)
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    // Windows has no directory handle fsync; MoveFileEx (std::fs::rename)
    // with the replaced file already flushed is the durable step there.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn replace_writes_exact_bytes_and_leaves_no_temp() -> TestResult {
        let dir = TempDir::new()?;
        let path = dir.path().join("data.json");
        std::fs::write(&path, "a much longer previous body than the next one")?;
        replace_file_atomically(&path, b"{}")?;
        assert_eq!(std::fs::read(&path)?, b"{}");
        let names: Vec<_> = std::fs::read_dir(dir.path())?
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names.len(), 1, "only the target remains: {names:?}");
        Ok(())
    }

    #[test]
    fn failed_replace_keeps_target_and_removes_temp() -> TestResult {
        let dir = TempDir::new()?;
        let target = dir.path().join("occupied");
        std::fs::create_dir(&target)?;
        std::fs::write(target.join("keep"), "x")?;
        assert!(replace_file_atomically(&target, b"{}").is_err());
        assert!(target.join("keep").exists());
        let names: Vec<_> = std::fs::read_dir(dir.path())?
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names.len(), 1, "temp file removed: {names:?}");
        Ok(())
    }

    #[test]
    fn other_thread_waits_for_the_lock() -> TestResult {
        let dir = TempDir::new()?;
        let held = SpecLock::acquire(dir.path())?;
        // TEST-042: no sleep. While held, the OS lock is exclusive to us...
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(held.dir().join(SPEC_LOCK_FILE_NAME))?;
        assert!(
            lock_file.try_lock_exclusive().is_err(),
            "the spec lock file is exclusively locked while held"
        );
        // ...so another thread's acquire is still waiting when we release it.
        let path = dir.path().to_path_buf();
        let acquired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (attempting, attempt) = mpsc::channel();
        let waiter = {
            let acquired = std::sync::Arc::clone(&acquired);
            thread::spawn(move || {
                let _ = attempting.send(());
                let got = SpecLock::acquire(&path).map(drop);
                acquired.store(true, std::sync::atomic::Ordering::SeqCst);
                got.is_ok()
            })
        };
        attempt.recv()?;
        assert!(
            !acquired.load(std::sync::atomic::Ordering::SeqCst),
            "the waiter cannot hold the lock while it is held"
        );
        drop(held);
        let ok = waiter.join().map_err(|_| "waiter thread panicked")?;
        assert!(ok, "the waiter gets the lock once it is released");
        Ok(())
    }
}
