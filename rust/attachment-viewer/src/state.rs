//! Shared viewer state — Clone newtype over `Arc<Inner>`, injected via the axum
//! `State` extractor (mirrors fspec.pro's `RelayState`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::ViewerConfig;

/// Cloneable handle to the viewer's shared state.
#[derive(Clone)]
pub struct ViewerState {
    inner: Arc<Inner>,
}

struct Inner {
    cwd: PathBuf,
}

impl ViewerState {
    /// Build state from a [`ViewerConfig`].
    pub fn new(config: ViewerConfig) -> Self {
        Self {
            inner: Arc::new(Inner { cwd: config.cwd }),
        }
    }

    /// The base directory that requests are confined to.
    pub fn cwd(&self) -> &Path {
        &self.inner.cwd
    }
}
