//! Viewer configuration — analogue of fspec.pro's `RelayConfig`.

use std::path::PathBuf;

/// Configuration for the attachment viewer server.
#[derive(Clone, Debug)]
pub struct ViewerConfig {
    /// Base directory that all `/view/{*path}` requests are resolved against and
    /// confined to (directory-traversal guard).
    pub cwd: PathBuf,
}
