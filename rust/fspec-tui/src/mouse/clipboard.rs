//! OSC 52 clipboard writer (COPY-001).
//!
//! Feature: spec/features/osc52-clipboard-writer.feature
//!
//! Writes selected transcript text to the system clipboard via the
//! terminal OSC 52 escape sequence, so copy works even while the TUI
//! holds mouse capture and over SSH — without native clipboard crates.
//!
//! Generic over `W: Write + Send` (mirroring
//! [`crate::mouse::MouseTrackingToggle`]) so unit tests can inject a
//! `Vec<u8>` and assert the EXACT bytes. Production code uses
//! [`Osc52Clipboard::with_stdout`] which delegates to `std::io::stdout()`.
//!
//! Byte format: `ESC ] 52 ; c ; <base64(utf8)> BEL`. The selection
//! target is `c` (the system clipboard) and the payload is the raw
//! UTF-8 bytes of the text, standard-base64 encoded with padding.
//!
//! Non-goals: no native clipboard crate (arboard), no OSC 52 READ/paste,
//! no chunking for very large payloads (terminals may cap OSC 52 size).

use std::io::{stdout, Write};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

/// Writes text to the system clipboard using the OSC 52 escape sequence.
///
/// Generic over `W: Write + Send` so tests can inject a `Vec<u8>` writer
/// and assert the exact escape sequence written.
pub struct Osc52Clipboard<W: Write + Send = std::io::Stdout> {
    writer: W,
}

impl Osc52Clipboard<std::io::Stdout> {
    /// Production constructor — writes to the real `stdout()`.
    pub fn with_stdout() -> Self {
        Self::new(stdout())
    }
}

impl<W: Write + Send> Osc52Clipboard<W> {
    /// Construct a clipboard writer that writes to `writer`.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Emit `ESC ] 52 ; c ; <base64(utf8)> BEL` for `text` and flush.
    pub fn copy(&mut self, text: &str) -> std::io::Result<()> {
        let payload = STANDARD.encode(text.as_bytes());
        self.writer.write_all(b"\x1b]52;c;")?;
        self.writer.write_all(payload.as_bytes())?;
        self.writer.write_all(b"\x07")?;
        self.writer.flush()
    }

    /// COPY-007 test seam: consume the clipboard and return the inner
    /// writer so tests can assert the exact bytes written.
    #[cfg(test)]
    pub fn into_writer_for_test(self) -> W {
        self.writer
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/osc52-clipboard-writer.feature
    use super::*;

    #[test]
    fn writing_ascii_text_emits_bel_terminated_osc52_sequence() {
        // @step Given an OSC 52 clipboard writer backed by an in-memory byte buffer
        let mut clip = Osc52Clipboard::new(Vec::<u8>::new());

        // @step When I copy the text "hi"
        clip.copy("hi").unwrap();

        // @step Then the buffer contains the bytes ESC ] 52 ; c ; aGk= BEL
        assert_eq!(clip.writer, b"\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn writing_empty_string_emits_empty_base64_payload() {
        // @step Given an OSC 52 clipboard writer backed by an in-memory byte buffer
        let mut clip = Osc52Clipboard::new(Vec::<u8>::new());

        // @step When I copy the empty string
        let result = clip.copy("");

        // @step Then the buffer contains the bytes ESC ] 52 ; c ; BEL
        assert_eq!(clip.writer, b"\x1b]52;c;\x07");

        // @step And the copy call does not panic
        assert!(result.is_ok());
    }

    #[test]
    fn writing_multi_line_text_base64_encodes_the_newline_byte() {
        // @step Given an OSC 52 clipboard writer backed by an in-memory byte buffer
        let mut clip = Osc52Clipboard::new(Vec::<u8>::new());

        // @step When I copy the text "a\nb"
        clip.copy("a\nb").unwrap();

        // @step Then the base64 payload in the buffer is "YQpi"
        assert_eq!(clip.writer, b"\x1b]52;c;YQpi\x07");
    }

    #[test]
    fn writing_an_emoji_encodes_its_raw_utf8_bytes() {
        // @step Given an OSC 52 clipboard writer backed by an in-memory byte buffer
        let mut clip = Osc52Clipboard::new(Vec::<u8>::new());

        // @step When I copy the emoji "😀"
        clip.copy("😀").unwrap();

        // @step Then the base64 payload in the buffer is "8J+YgA=="
        assert_eq!(clip.writer, b"\x1b]52;c;8J+YgA==\x07");
    }

    #[test]
    fn production_constructor_targets_stdout_and_test_constructor_targets_a_buffer() {
        // @step Given the production constructor with_stdout is available
        let _production: fn() -> Osc52Clipboard<std::io::Stdout> = Osc52Clipboard::with_stdout;

        // @step And the test constructor new accepts any Write sink
        let mut clip = Osc52Clipboard::new(Vec::<u8>::new());

        // @step When I construct a writer over an in-memory byte buffer and copy "hi"
        clip.copy("hi").unwrap();

        // @step Then the exact bytes are captured in that buffer without touching the real terminal
        assert_eq!(clip.writer, b"\x1b]52;c;aGk=\x07");
    }
}
