//! NAPI persistence facade — pure adapter over `codelet_core::persistence`.
//!
//! RPC-031..RPC-034 lifted every persistence type, singleton, and free
//! function into `codelet_core::persistence`. RPC-035 reduces this module
//! to a ~15-line facade: the flat re-export below preserves every
//! historical `crate::persistence::Foo` import inside codelet-napi, and
//! the conditional `napi_bindings` module is the only place a `#[napi]`
//! attribute appears in the persistence surface.

pub use codelet_core::persistence::*;

#[cfg(not(feature = "noop"))]
mod napi_bindings;

#[cfg(not(feature = "noop"))]
pub use napi_bindings::*;

/// Rust-friendly wrapper around the data-directory configuration.
///
/// Mirrors the body of [`napi_bindings::persistence_set_data_directory`]
/// so internal Rust callers (tests, integration suites) can configure
/// the data directory without crossing the NAPI boundary. Resets every
/// lifted persistence singleton plus the napi-side credentials and
/// knowledge-graph stores.
pub fn set_data_directory(dir: std::path::PathBuf) -> Result<(), String> {
    codelet_common::set_data_directory(dir)?;
    codelet_core::persistence::reset_stores_for_tests();
    #[cfg(not(feature = "noop"))]
    {
        crate::credentials::reset_credential_store();
        crate::graph::reset_graph_db();
    }
    Ok(())
}
