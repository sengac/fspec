//! TUI-126: shared clipboard and URL-launcher seams for copy tests.
//!
//! Every copy test drives the App from the key event to the OSC 52 bytes the
//! App writes (`App::set_clipboard_writer_for_test`), and every open test
//! records what the App's URL launcher was asked to open
//! (`App::set_url_launcher_for_test`).

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::io::Write;
use std::sync::{Arc, Mutex};

use codelet_fspec_tui::App;
use codelet_test_helpers::{OrFail, TestError};

/// A writer whose bytes the test can read back.
pub struct SharedWriter(pub Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| std::io::Error::other(format!("clipboard buffer mutex: {error}")))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The App's clipboard output, captured.
#[derive(Clone)]
pub struct Clipboard(Arc<Mutex<Vec<u8>>>);

impl Clipboard {
    /// Capture `app`'s clipboard writes from now on.
    pub fn attach(app: &mut App) -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        app.set_clipboard_writer_for_test(Box::new(SharedWriter(buffer.clone())));
        Self(buffer)
    }

    /// Every byte written so far.
    pub fn bytes(&self) -> Result<Vec<u8>, TestError> {
        Ok(self.0.lock().or_fail("clipboard buffer mutex")?.clone())
    }

    /// Forget what was written so far.
    pub fn clear(&self) -> Result<(), TestError> {
        self.0.lock().or_fail("clipboard buffer mutex")?.clear();
        Ok(())
    }
}

/// The OSC 52 sequence the App writes for `text`: `ESC ] 52 ; c ; <base64> BEL`.
pub fn osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

/// A URL launcher that records every URL and answers with `result`.
#[derive(Clone)]
pub struct Launcher {
    opened: Arc<Mutex<Vec<String>>>,
    result: Arc<Mutex<Result<(), String>>>,
}

impl Launcher {
    /// Install a recording launcher on `app` that succeeds.
    pub fn attach(app: &mut App) -> Self {
        let launcher = Self {
            opened: Arc::new(Mutex::new(Vec::new())),
            result: Arc::new(Mutex::new(Ok(()))),
        };
        let opened = launcher.opened.clone();
        let result = launcher.result.clone();
        app.set_url_launcher_for_test(Arc::new(move |url: &str| {
            opened
                .lock()
                .map_err(|error| error.to_string())?
                .push(url.to_string());
            result.lock().map_err(|error| error.to_string())?.clone()
        }));
        launcher
    }

    /// Make every later launch fail with `reason`.
    pub fn fail_with(&self, reason: &str) -> Result<(), TestError> {
        *self.result.lock().or_fail("launcher result mutex")? = Err(reason.to_string());
        Ok(())
    }

    /// Every URL the App asked to open, in order.
    pub fn opened(&self) -> Result<Vec<String>, TestError> {
        Ok(self.opened.lock().or_fail("launcher mutex")?.clone())
    }
}
