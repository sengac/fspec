//! BUG-610: every panic is written to the process log before any other
//! panic hook runs.
//!
//! The daemon's and the TUI's panic hooks used to restore the terminal and
//! chain to the default hook, which prints to stderr: a terminal nobody
//! keeps once the TUI is gone. A panic that poisoned the daemon's session
//! store was therefore never recorded. [`wrap`] builds a hook that logs the
//! panic at ERROR (thread, location, message and, when the panic happens
//! inside a session's agent loop, the session id) and then runs the
//! caller's terminal restore and the previous hook.
//!
//! Several wrapped hooks can be chained (the fspec binary and fspec-tui
//! each install one); a panic is logged once, by the outermost.

use std::cell::Cell;
use std::fmt::Display;
use std::future::Future;
use std::panic::PanicHookInfo;
use std::sync::Once;

/// The shape of a `std::panic` hook.
pub type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

tokio::task_local! {
    static SESSION: String;
}

thread_local! {
    /// True while a wrapped hook is running on this thread, so the hooks it
    /// chains to do not log the same panic again.
    static LOGGING: Cell<bool> = const { Cell::new(false) };
}

/// Run `future` as work of `session_id`: a panic inside it is logged with
/// that session id.
pub async fn in_session<F: Future>(session_id: impl Display, future: F) -> F::Output {
    SESSION.scope(session_id.to_string(), future).await
}

/// [`in_session`] for synchronous work on the current thread.
pub fn in_session_sync<R>(session_id: impl Display, operation: impl FnOnce() -> R) -> R {
    SESSION.sync_scope(session_id.to_string(), operation)
}

/// A hook that logs the panic, then runs `before_previous` (e.g. restoring
/// the terminal), then `previous`.
pub fn wrap(previous: PanicHook, before_previous: impl Fn() + Sync + Send + 'static) -> PanicHook {
    Box::new(move |info| {
        let outermost = !LOGGING.with(|logging| logging.replace(true));
        if outermost {
            log(info);
        }
        before_previous();
        previous(info);
        if outermost {
            LOGGING.with(|logging| logging.set(false));
        }
    })
}

/// Wrap the current hook with [`wrap`] once per process, for processes
/// that have no terminal to restore (the headless daemon).
pub fn install() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| std::panic::set_hook(wrap(std::panic::take_hook(), || {})));
}

fn log(info: &PanicHookInfo<'_>) {
    let thread = std::thread::current();
    let thread = thread.name().unwrap_or("<unnamed>");
    let location = info.location().map_or_else(
        || "<unknown location>".to_string(),
        |location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        },
    );
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>");
    // RUST_BACKTRACE / RUST_LIB_BACKTRACE decide, as for the default hook.
    let backtrace = std::backtrace::Backtrace::capture();
    let backtrace = (backtrace.status() == std::backtrace::BacktraceStatus::Captured)
        .then(|| backtrace.to_string());
    let session_id = SESSION.try_with(Clone::clone).ok();
    tracing::error!(
        target: "panic",
        thread = %thread,
        location = %location,
        session_id = session_id.as_deref().map(tracing::field::display),
        backtrace = backtrace.as_deref().map(tracing::field::display),
        "panicked: {message}"
    );
}
