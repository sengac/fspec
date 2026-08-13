//! Debounced "let the terminal handle native text selection" toggle.
//!
//! Feature: spec/features/mouse-tracking-toggle.feature
//! Card: RPC-023 (scaffolding for RPC-019).
//!
//! TS reference: `src/tui/components/VirtualList.tsx` lines 180–234,
//! 540–572 — `temporarilyDisableMouseTracking` + a 5-second
//! re-enable timer + immediate re-enable on button-release.
//!
//! Generic over `W: Write + Send` per Decision Q6 so unit tests can
//! inject a `Vec<u8>` (or any `SharedBuf`-style adapter) and assert
//! the EXACT byte sequence written for `DisableMouseCapture` and
//! `EnableMouseCapture`. Production code uses
//! [`MouseTrackingToggle::with_stdout`] which delegates to `std::io::stdout()`.
//!
//! This is the ONLY non-`terminal.rs` module that is permitted to call
//! [`EnableMouseCapture`] / [`DisableMouseCapture`] — the source-shape
//! suite (`tests/source_shape_rpc023.rs`) enforces that locality.
//!
//! Scope note (Decision Q9): the BoardView slice (RPC-023) does NOT
//! wire any button-press path through this toggle. RPC-019 owns the
//! VirtualList scrollback wiring + acceleration.

use std::io::{stdout, Write};
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::components::Action;

/// Re-enable mouse capture this many seconds after the last
/// button-down event. Matches the TS `setTimeout(..., 5000)` in
/// `VirtualList.tsx`.
const REENABLE_AFTER: Duration = Duration::from_secs(5);

/// Coordinates the "`DisableMouseCapture` during text selection,
/// `EnableMouseCapture` when the user is done" lifecycle.
///
/// Generic over `W: Write + Send` so tests can inject a `Vec<u8>` /
/// `Arc<Mutex<Vec<u8>>>` writer and assert the exact escape sequences.
pub struct MouseTrackingToggle<W: Write + Send = std::io::Stdout> {
    writer: W,
    disabled: bool,
    re_enable_handle: Option<JoinHandle<()>>,
    action_tx: UnboundedSender<Action>,
    owner_id: String,
}

impl MouseTrackingToggle<std::io::Stdout> {
    /// Production constructor — writes to the real `stdout()`.
    pub fn with_stdout(owner_id: impl Into<String>, action_tx: UnboundedSender<Action>) -> Self {
        Self::new(stdout(), owner_id, action_tx)
    }
}

impl<W: Write + Send> MouseTrackingToggle<W> {
    /// Construct a toggle that writes to `writer`. The toggle starts in
    /// the ENABLED state (i.e. capture is presumed already on per the
    /// `TerminalGuard::init` lifecycle).
    pub fn new(writer: W, owner_id: impl Into<String>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            writer,
            disabled: false,
            re_enable_handle: None,
            action_tx,
            owner_id: owner_id.into(),
        }
    }

    /// Whether the toggle is currently in the disabled state — i.e.
    /// mouse capture was turned off so the terminal can run native text
    /// selection.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Button-down handler. Writes [`DisableMouseCapture`] immediately
    /// (idempotent when already disabled) and (re)schedules the
    /// 5-second timer that fires
    /// [`Action::ReEnableMouseTracking`]`(owner)` onto the action bus.
    ///
    /// Each call cancels any pending re-enable timer first — repeated
    /// button presses extend the debounce window.
    pub fn temporarily_disable(&mut self) {
        self.cancel_pending_reenable();
        if !self.disabled {
            let _ = execute!(self.writer, DisableMouseCapture);
            self.disabled = true;
        }
        let tx = self.action_tx.clone();
        let owner = self.owner_id.clone();
        self.re_enable_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(REENABLE_AFTER).await;
            let _ = tx.send(Action::ReEnableMouseTracking(owner));
        }));
    }

    /// Button-release handler. Cancels the pending re-enable timer and
    /// writes [`EnableMouseCapture`] immediately so the wheel works on
    /// the next event. Idempotent when already enabled — no further
    /// bytes are written.
    pub fn re_enable(&mut self) {
        self.cancel_pending_reenable();
        if self.disabled {
            let _ = execute!(self.writer, EnableMouseCapture);
            self.disabled = false;
        }
    }

    fn cancel_pending_reenable(&mut self) {
        if let Some(handle) = self.re_enable_handle.take() {
            handle.abort();
        }
    }
}

impl<W: Write + Send> Drop for MouseTrackingToggle<W> {
    fn drop(&mut self) {
        self.cancel_pending_reenable();
        if self.disabled {
            let _ = execute!(self.writer, EnableMouseCapture);
            self.disabled = false;
        }
    }
}
