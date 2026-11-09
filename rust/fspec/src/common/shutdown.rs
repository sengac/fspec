// Shutdown signals: SIGINT/SIGTERM drain, SIGHUP rebuild (RPC-011;
// REFAC-024: moved out of common.rs).

/// RPC-011 enum return for `build_shutdown_future` — distinguishes
/// SIGINT/SIGTERM (drain + exit) from SIGHUP (rebuild watcher, keep
/// running).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Sigint,
    // Only constructed by the `#[cfg(unix)]` signal handlers, so these two
    // variants are dead code on non-unix targets (e.g. Windows cross-builds).
    #[cfg_attr(not(unix), allow(dead_code))]
    Sigterm,
    #[cfg_attr(not(unix), allow(dead_code))]
    Sighup,
}

/// Future that completes on SIGINT (ctrl_c), SIGTERM, or SIGHUP. Unix uses the
/// tokio::signal::unix handlers. RPC-011 rule [25] / scenario "SIGHUP
/// rebuilds...": SIGHUP yields `ShutdownReason::Sighup` so the daemon's
/// top-level signal loop can rebuild the watcher and keep running.
///
/// NOTE — race-with-signal limitation: this function installs the
/// tokio signal listeners SYNCHRONOUSLY when called, then returns an
/// `impl Future` that awaits one of them. A signal that arrives BEFORE
/// the first call is dropped via the default handler (process exits
/// with signal code instead of clean exit 0). For long-running daemons,
/// use [`ShutdownSignals::install`] FIRST and call `.next().await` in
/// a loop — that arms the listeners up-front so the bootstrap window
/// is safe.
#[cfg(unix)]
pub fn build_shutdown_future() -> impl std::future::Future<Output = Result<ShutdownReason>> {
    use tokio::signal::unix::{signal, SignalKind};
    let sigterm_result = signal(SignalKind::terminate());
    let sigint_result = signal(SignalKind::interrupt());
    let sighup_result = signal(SignalKind::hangup());
    async move {
        let mut sigterm = sigterm_result?;
        let mut sigint = sigint_result?;
        let mut sighup = sighup_result?;
        tokio::select! {
            _ = sigterm.recv() => Ok(ShutdownReason::Sigterm),
            _ = sigint.recv() => Ok(ShutdownReason::Sigint),
            _ = sighup.recv() => Ok(ShutdownReason::Sighup),
        }
    }
}

#[cfg(not(unix))]
pub fn build_shutdown_future() -> impl std::future::Future<Output = Result<ShutdownReason>> {
    async {
        tokio::signal::ctrl_c().await?;
        Ok(ShutdownReason::Sigint)
    }
}

/// RPC-011: long-lived shutdown-signal handles. Construct via
/// [`ShutdownSignals::install`] BEFORE any work the test harness can
/// race a signal against (in particular before `bind_and_serve`'s
/// async port-bind). Drop only when the daemon is exiting — the
/// underlying `tokio::signal::unix::Signal` registrations are removed
/// on drop, so dropping mid-loop creates a fresh race window.
///
/// Use [`next`](Self::next) in the daemon's top-level loop:
/// SIGINT/SIGTERM should `break`, SIGHUP should `continue` (rebuild
/// watcher and re-await).
#[cfg(unix)]
pub struct ShutdownSignals {
    sigterm: tokio::signal::unix::Signal,
    sigint: tokio::signal::unix::Signal,
    sighup: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    /// Install tokio listeners for SIGTERM, SIGINT, SIGHUP. Returns
    /// `Err` if any of them fails to register (extremely unlikely on a
    /// healthy unix process).
    pub fn install() -> Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        Ok(Self {
            sigterm: signal(SignalKind::terminate())?,
            sigint: signal(SignalKind::interrupt())?,
            sighup: signal(SignalKind::hangup())?,
        })
    }

    /// Await one of the three signals and yield the corresponding
    /// [`ShutdownReason`]. Re-callable in a loop — each call awaits
    /// the NEXT signal.
    pub async fn next(&mut self) -> ShutdownReason {
        tokio::select! {
            _ = self.sigterm.recv() => ShutdownReason::Sigterm,
            _ = self.sigint.recv() => ShutdownReason::Sigint,
            _ = self.sighup.recv() => ShutdownReason::Sighup,
        }
    }
}

/// Windows fallback: only ctrl_c is supported (mirrors
/// [`build_shutdown_future`]).
#[cfg(not(unix))]
pub struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    pub fn install() -> Result<Self> {
        Ok(Self)
    }
    pub async fn next(&mut self) -> ShutdownReason {
        let _ = tokio::signal::ctrl_c().await;
        ShutdownReason::Sigint
    }
}
