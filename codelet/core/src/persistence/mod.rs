//! Persistence module — shared between codelet_napi (NAPI exports) and
//! codelet_rpc (FspecService RPC methods).
//!
//! Lifted from codelet/napi/src/persistence in RPC-025 so codelet_rpc can
//! call into the history store without re-introducing a `rpc → napi`
//! dependency.
//!
//! Currently exports only the history-related surface (HistoryStore,
//! HistoryEntry, plus add/get/search helpers); the rest of the NAPI
//! persistence module (MessageStore, SessionStore, BlobStore, etc.) stays
//! in codelet/napi and is not yet lifted.

pub mod history;
pub mod sessions;

pub use history::{add, get, search, HistoryEntry, HistoryStore, PastedContent};
pub use sessions::delete_session;
