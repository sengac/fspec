//! Shared helper for rendering `std::io::Error` with Node-`libuv`-style text.
//!
//! The TypeScript fspec CLI surfaces filesystem failures via Node's `fs`
//! promises, whose `ENOENT` errors read `ENOENT: no such file or directory,
//! open '<path>'`. Several ported commands need to reproduce that exact text
//! for byte-for-byte parity, so the formatter lives here as a single
//! source-of-truth instead of being copy-pasted per command (DRY).

/// Render an [`std::io::Error`] the way the Node.js `fs` layer does.
///
/// For a not-found error this yields `ENOENT: no such file or directory, open
/// '<path>'` (matching libuv); for every other kind it falls back to the
/// platform `Display` text.
pub fn format_io_error(e: &std::io::Error, path: &str) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        format!("ENOENT: no such file or directory, open '{path}'")
    } else {
        format!("{e}")
    }
}
