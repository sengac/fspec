//! TEST-042: the one timed wait fspec's tests may use.
//!
//! Feature: spec/features/fspec-tests-never-wait-on-wall-clock-sleeps-or-fixed-timeouts.feature
//!
//! A test waits on the event or state it asserts. When that event never
//! comes (a deadlock or a lost message), the test must still end, loudly:
//! [`deadlock_guard`] and [`deadlock_guard_blocking`] give up after the
//! generous [`DEADLOCK_GUARD`] and return a [`Failure`] naming what was
//! awaited. The limit is never a correctness condition: a test whose event
//! arrives under any load passes, however slow the machine.
//!
//! This file is the only place a timed wait exists in fspec's test code. It
//! uses a std thread and channel, never tokio (RPC-067: test-helpers does
//! not depend on tokio), so it works under any async runtime.

use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::Failure;

/// How long a guarded wait may take before the test fails: long enough that
/// no load ever reaches it, short enough that a deadlocked suite still ends.
pub const DEADLOCK_GUARD: Duration = Duration::from_secs(300);

/// Await `future`, failing with a message naming `what` if it has not
/// completed within [`DEADLOCK_GUARD`].
///
/// ```ignore
/// let reply = deadlock_guard("the reply", rx.recv()).await?;
/// ```
pub fn deadlock_guard<F: Future>(what: &str, future: F) -> DeadlockGuard<F> {
    guard_within(what, DEADLOCK_GUARD, future)
}

/// Run `work` on its own thread and wait for its result, failing with a
/// message naming `what` if it has not returned within [`DEADLOCK_GUARD`]
/// (or panicked).
pub fn deadlock_guard_blocking<T, W>(what: &str, work: W) -> Result<T, Failure>
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
{
    blocking_within(what, DEADLOCK_GUARD, work)
}

fn expired(what: &str, limit: Duration) -> Failure {
    Failure::new(format!(
        "deadlock guard: {what} did not finish within {limit:?}"
    ))
}

pub(crate) fn guard_within<F: Future>(what: &str, limit: Duration, future: F) -> DeadlockGuard<F> {
    DeadlockGuard {
        future: Box::pin(future),
        what: what.to_string(),
        limit,
        watcher: None,
    }
}

pub(crate) fn blocking_within<T, W>(what: &str, limit: Duration, work: W) -> Result<T, Failure>
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
{
    let (done, result) = mpsc::channel();
    std::thread::spawn(move || {
        // The receiver is gone only when the guard already failed.
        let _ = done.send(work());
    });
    match result.recv_timeout(limit) {
        Ok(value) => Ok(value),
        Err(RecvTimeoutError::Timeout) => Err(expired(what, limit)),
        Err(RecvTimeoutError::Disconnected) => {
            Err(Failure::new(format!("deadlock guard: {what} panicked")))
        }
    }
}

/// The future [`deadlock_guard`] returns.
pub struct DeadlockGuard<F> {
    future: Pin<Box<F>>,
    what: String,
    limit: Duration,
    watcher: Option<Watcher>,
}

/// The thread that marks the guard expired and wakes the task. Dropping the
/// guard drops `_stop`, which ends the thread at once.
struct Watcher {
    state: Arc<Mutex<WatchState>>,
    _stop: mpsc::Sender<()>,
}

#[derive(Default)]
struct WatchState {
    expired: bool,
    waker: Option<Waker>,
}

impl Watcher {
    fn start(limit: Duration) -> Self {
        let state = Arc::new(Mutex::new(WatchState::default()));
        let (stop, stopped) = mpsc::channel::<()>();
        let watched = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(limit) {
                let mut state = watched.lock().unwrap_or_else(PoisonError::into_inner);
                state.expired = true;
                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
            }
        });
        Self { state, _stop: stop }
    }
}

impl<F: Future> Future for DeadlockGuard<F> {
    type Output = Result<F::Output, Failure>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Poll::Ready(value) = this.future.as_mut().poll(cx) {
            return Poll::Ready(Ok(value));
        }
        let limit = this.limit;
        let watcher = this.watcher.get_or_insert_with(|| Watcher::start(limit));
        let mut state = watcher.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.expired {
            return Poll::Ready(Err(expired(&this.what, limit)));
        }
        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// Wait until `check` reports the state a test asserts, polling it between
/// yields to the executor (no sleep), under the deadlock guard. For a state
/// with no event to await (a status read, a file a child writes); prefer
/// awaiting the event itself when there is one.
pub async fn wait_until<F, Fut>(what: &str, mut check: F) -> Result<(), Failure>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    deadlock_guard(what, async move {
        while !check().await {
            yield_now().await;
        }
    })
    .await
}

/// [`wait_until`] for a blocking check, run on its own thread.
pub fn wait_until_blocking<F>(what: &str, mut check: F) -> Result<(), Failure>
where
    F: FnMut() -> bool + Send + 'static,
{
    deadlock_guard_blocking(what, move || {
        while !check() {
            std::thread::yield_now();
        }
    })
}

/// Poll `future` until `ready` reports the state the test asserts while the
/// future is still pending; fail if the future completes first. For "it is
/// still waiting" checks: the future must stay pending until the state that
/// proves it is waiting (a recovery status, a request the server received)
/// is reached, with no waiting window, under the deadlock guard.
pub async fn pending_until<F, C>(what: &str, future: &mut F, mut ready: C) -> Result<(), Failure>
where
    F: Future + Unpin,
    C: FnMut() -> bool,
{
    let message = format!("the awaited future completed before {what}");
    deadlock_guard(what, async move {
        loop {
            if ready() {
                return Ok(());
            }
            let completed =
                std::future::poll_fn(|cx| Poll::Ready(Pin::new(&mut *future).poll(cx).is_ready()))
                    .await;
            if completed {
                return Err(Failure::new(message));
            }
            yield_now().await;
        }
    })
    .await?
}

/// [`wait_until_blocking`] on the calling thread, for a check that borrows
/// local state (a child process, a path): polled between thread yields,
/// failing once [`DEADLOCK_GUARD`] has passed.
pub fn wait_until_here<F>(what: &str, check: F) -> Result<(), Failure>
where
    F: FnMut() -> bool,
{
    here_within(what, DEADLOCK_GUARD, check)
}

pub(crate) fn here_within<F>(what: &str, limit: Duration, mut check: F) -> Result<(), Failure>
where
    F: FnMut() -> bool,
{
    let started = std::time::Instant::now();
    while !check() {
        if started.elapsed() >= limit {
            return Err(expired(what, limit));
        }
        std::thread::yield_now();
    }
    Ok(())
}

/// Yield once to the executor (runtime-agnostic: no tokio in test-helpers).
pub async fn yield_now() {
    struct YieldNow(bool);
    impl Future for YieldNow {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                return Poll::Ready(());
            }
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
    YieldNow(false).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal executor: park the thread until the waker fires.
    fn block_on<F: Future>(future: F) -> F::Output {
        struct ThreadWaker(std::thread::Thread);
        impl std::task::Wake for ThreadWaker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
            std::thread::park();
        }
    }

    #[test]
    fn the_deadlock_guard_returns_the_awaited_value_or_fails_loudly() -> crate::TestResult {
        // @step Given a future that completes and a future that never completes
        let completes = std::future::ready(42);
        let never = std::future::pending::<()>();
        // @step When each is awaited through deadlock_guard with a short guard
        let value = block_on(deadlock_guard("the answer", completes));
        let failure = block_on(guard_within("the reply", Duration::from_millis(20), never));
        // @step Then the completed future's value is returned
        assert_eq!(value?, 42);
        // @step And the never-completing one fails naming what was awaited and the guard duration
        let message = match failure {
            Ok(()) => return Err("a future that never completes cannot finish".into()),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(
            message,
            "deadlock guard: the reply did not finish within 20ms"
        );
        let blocked = blocking_within("the worker", Duration::from_millis(20), || {
            let (_keep, never) = mpsc::channel::<()>();
            let _ = never.recv();
        });
        assert!(blocked.is_err());
        assert_eq!(deadlock_guard_blocking("the sum", || 2 + 2)?, 4);
        // A state is waited on by polling it between yields.
        let mut polls = 0;
        block_on(wait_until("the third poll", || {
            polls += 1;
            std::future::ready(polls == 3)
        }))?;
        assert_eq!(polls, 3);
        let mut tries = 0;
        wait_until_blocking("the third try", move || {
            tries += 1;
            tries == 3
        })?;
        // A pending future passes once the state is reached; a completed one fails.
        let mut pending = std::future::pending::<()>();
        let mut checks = 0;
        block_on(pending_until("the second check", &mut pending, || {
            checks += 1;
            checks == 2
        }))?;
        let mut ready = std::future::ready(());
        assert!(block_on(pending_until("never", &mut ready, || false)).is_err());
        let mut rounds = 0;
        wait_until_here("the third round", || {
            rounds += 1;
            rounds == 3
        })?;
        assert!(here_within("never", Duration::from_millis(20), || false).is_err());
        // The real guard is generous: no load reaches it.
        assert_eq!(DEADLOCK_GUARD, Duration::from_secs(300));
        Ok(())
    }
}
