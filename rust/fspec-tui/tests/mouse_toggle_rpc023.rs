//! RPC-023 — MouseTrackingToggle unit tests.
//!
//! Feature: spec/features/mouse-tracking-toggle.feature
//!
//! Pins the TUI-078 scaffolding: writer injection (Vec<u8> via shared
//! handle), exact DisableMouseCapture / EnableMouseCapture escape bytes,
//! 5-second debounce timer with restart, Drop re-enable.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use codelet_fspec_tui::mouse::MouseTrackingToggle;
use codelet_fspec_tui::Action;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use tokio::sync::mpsc::unbounded_channel;

/// Writable handle that the test can also peek at — Arc<Mutex<Vec<u8>>>
/// implements `Write` via this newtype.
#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl SharedBuf {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The exact bytes crossterm writes for `DisableMouseCapture` — sniff
/// them from the crate at test-time so the assertion stays accurate if
/// crossterm changes its enable/disable sequence (it currently flips
/// SGR-1006 + URXVT-1015 + BUTTON_EVENT-1002 + ANY-EVENT-1003 + ALL-1000).
fn disable_bytes() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    execute!(buf, DisableMouseCapture).expect("execute!");
    buf
}

/// The exact bytes crossterm writes for `EnableMouseCapture`.
fn enable_bytes() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    execute!(buf, EnableMouseCapture).expect("execute!");
    buf
}

/// Scenario: MouseTrackingToggle::temporarily_disable writes DisableMouseCapture bytes
#[tokio::test(start_paused = true)]
async fn temporarily_disable_writes_disable_mouse_capture_bytes() {
    // @step Given a MouseTrackingToggle constructed with a Vec<u8> writer
    let buf = SharedBuf::new();
    // @step And an UnboundedSender<Action> bound to a test receiver
    let (tx, _rx) = unbounded_channel::<Action>();
    let mut toggle = MouseTrackingToggle::new(buf.clone(), "test-owner", tx);

    // @step When temporarily_disable is called
    toggle.temporarily_disable();

    // @step Then the Vec<u8> writer contains the exact DisableMouseCapture escape bytes "\x1b[?1006l\x1b[?1000l"
    assert_eq!(
        buf.bytes(),
        disable_bytes(),
        "writer must contain the crossterm DisableMouseCapture escape sequence"
    );
    // @step And is_disabled() returns true
    assert!(toggle.is_disabled());
}

/// Scenario: MouseTrackingToggle::re_enable writes EnableMouseCapture bytes once
#[tokio::test(start_paused = true)]
async fn re_enable_writes_enable_mouse_capture_bytes_once() {
    // @step Given a MouseTrackingToggle whose temporarily_disable has already run against a Vec<u8> writer
    let buf = SharedBuf::new();
    let (tx, _rx) = unbounded_channel::<Action>();
    let mut toggle = MouseTrackingToggle::new(buf.clone(), "test-owner", tx);
    toggle.temporarily_disable();
    let after_disable = buf.bytes().len();

    // @step When re_enable is called
    toggle.re_enable();

    // @step Then the writer's tail bytes match the EnableMouseCapture escape "\x1b[?1000h\x1b[?1006h"
    let bytes = buf.bytes();
    let expected_enable = enable_bytes();
    assert!(
        bytes.len() >= after_disable + expected_enable.len(),
        "writer must have grown by the EnableMouseCapture escape"
    );
    assert_eq!(
        &bytes[after_disable..],
        &expected_enable[..],
        "tail bytes must match the EnableMouseCapture escape"
    );
    // @step And is_disabled() returns false
    assert!(!toggle.is_disabled());

    let len_before_second_call = buf.bytes().len();
    // @step When re_enable is called a second time while already enabled
    toggle.re_enable();
    // @step Then no further bytes are written to the writer
    assert_eq!(
        buf.bytes().len(),
        len_before_second_call,
        "second re_enable on already-enabled toggle must not write"
    );
}

/// Scenario: MouseTrackingToggle::Drop re-enables capture when still disabled
#[tokio::test(start_paused = true)]
async fn drop_re_enables_capture_when_still_disabled() {
    // @step Given a MouseTrackingToggle constructed with a Vec<u8> writer
    let buf = SharedBuf::new();
    let (tx, _rx) = unbounded_channel::<Action>();
    let len_before;
    {
        let mut toggle = MouseTrackingToggle::new(buf.clone(), "test-owner", tx);
        // @step And temporarily_disable has been called so disabled is true
        toggle.temporarily_disable();
        len_before = buf.bytes().len();
        // @step When the toggle is dropped
    }
    // @step Then the EnableMouseCapture escape bytes are written to the writer during Drop
    let bytes = buf.bytes();
    let expected_enable = enable_bytes();
    assert!(
        bytes.len() >= len_before + expected_enable.len(),
        "Drop must have grown the writer by EnableMouseCapture"
    );
    assert_eq!(
        &bytes[len_before..],
        &expected_enable[..],
        "Drop must write the EnableMouseCapture escape"
    );
}

/// Scenario: Repeated temporarily_disable restarts the debounce timer
#[tokio::test(start_paused = true)]
async fn repeated_temporarily_disable_restarts_the_debounce_timer() {
    // @step Given a MouseTrackingToggle with tokio::time paused
    let buf = SharedBuf::new();
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut toggle = MouseTrackingToggle::new(buf, "scrollback", tx);

    // @step When temporarily_disable is called
    toggle.temporarily_disable();
    // @step And four seconds of virtual time elapse
    tokio::time::sleep(Duration::from_secs(4)).await;
    // @step And temporarily_disable is called again
    toggle.temporarily_disable();
    // @step And another four seconds of virtual time elapse
    tokio::time::sleep(Duration::from_secs(4)).await;
    // @step Then no Action::ReEnableMouseTracking has been emitted yet
    assert!(
        rx.try_recv().is_err(),
        "no Action::ReEnableMouseTracking should be delivered before the 5s timer (from the SECOND call) elapses"
    );
    // @step When two more seconds of virtual time elapse
    tokio::time::sleep(Duration::from_secs(2)).await;
    // @step Then exactly one Action::ReEnableMouseTracking is delivered to the receiver
    let action = rx
        .recv()
        .await
        .expect("Action::ReEnableMouseTracking expected");
    match action {
        Action::ReEnableMouseTracking(owner) => assert_eq!(owner, "scrollback"),
        other => panic!("expected ReEnableMouseTracking, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "exactly one ReEnableMouseTracking should fire after the FINAL 5s window"
    );
}
