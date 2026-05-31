//! Persistence module — shared between codelet_napi (NAPI exports),
//! codelet_rpc (FspecService RPC methods), codelet_rpc_embedded,
//! codelet_rpc_server, and the upcoming codelet_sessions crate.
//!
//! Lifted from codelet/napi/src/persistence in stages:
//! - RPC-025: history.rs (HistoryStore + add/get/search)
//! - RPC-026: sessions.rs (delete_session helper)
//! - RPC-031: message_envelope.rs (Claude-Code wire envelope schema)
//! - RPC-032: messages.rs (MessageStore + binary index)
//! - RPC-033: manifest.rs (SessionStore + SessionManifest + every
//!   session-level free function; MESSAGE_STORE + SESSION_STORE singletons)
//! - RPC-034: blob.rs + blob_processing.rs (BlobStore + BLOB_STORE
//!   singleton + store_blob/get_blob/blob_exists facade +
//!   should_use_blob_storage + BLOB_REF_PREFIX + envelope blob
//!   processing/rehydration)
//!
//! Phase 1 of RPC-030 is now complete after RPC-035: codelet-napi no
//! longer owns any pure-Rust persistence logic OR persistence test
//! ownership. Its `persistence/mod.rs` is reduced to a ~15-line facade
//! (doc-comment + `pub use codelet_core::persistence::*;` + the
//! `#[cfg(not(feature = "noop"))] mod napi_bindings;` gate). The
//! credentials + knowledge-graph reset that previously lived in the
//! NAPI `set_data_directory` wrapper is inlined into the
//! `#[napi] persistence_set_data_directory` entry point in
//! `codelet/napi/src/persistence/napi_bindings.rs`. The 48-test +
//! 9-test persistence suites that previously lived under codelet-napi
//! now live in `codelet/core/src/persistence/{tests,lazy_init_tests}.rs`.

pub mod blob;
pub mod blob_processing;
pub mod history;
pub mod manifest;
pub mod message_envelope;
pub mod messages;
pub mod sessions;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lazy_init_tests;

pub use blob::*;
pub use blob_processing::*;
pub use history::{add, get, search, HistoryEntry, HistoryStore, PastedContent};
pub use manifest::*;
pub use message_envelope::*;
pub use messages::{compute_hash, MessageRef, MessageSource, MessageStore, StoredMessage};
// RPC-033: `delete_session` now lives canonically in `manifest`. The
// `sessions` module's helper stays as a thin alias for backward compat
// with the RPC-026 cross-transport parity test that imports it via
// `codelet_core::persistence::sessions::delete_session`.

/// Eagerly create all persistence subdirectories under the configured
/// data directory: `{data_dir}/messages/`, `{data_dir}/sessions/`, and
/// `{data_dir}/blobs/`.
///
/// The lifted [`MessageStore::new`], [`manifest::SessionStore::new`]
/// and [`blob::BlobStore::new`] each create their own subdirectory
/// lazily on first use, but a handful of NAPI test fixtures need the
/// directories to exist BEFORE the first store call so they can
/// `chmod` / inspect them. This helper is the codelet-core delegate
/// that `codelet_napi::persistence::ensure_directories` routes through
/// — per RPC-034 rule [7], the NAPI shim is a thin shim that delegates
/// to this codelet-core helper.
pub fn ensure_directories() -> Result<(), String> {
    let base = codelet_common::get_data_dir()?;
    for sub in ["messages", "sessions", "blobs"] {
        let dir = base.join(sub);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create directory {dir:?}: {e}"))?;
    }
    Ok(())
}
