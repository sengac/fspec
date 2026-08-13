//! Credential Management Module
//!
//! Implements credential resolution for provider API keys with support for:
//! - Credentials file (~/.fspec/credentials/credentials.json)
//! - Environment variables
//! - Project .env files
//!
//! This module is the single source of truth for credential resolution.
//! Lifted to `codelet-sessions` by **RPC-040** from
//! `rust/napi/src/credentials/`. The NAPI binding lives separately
//! in `rust/napi/src/credentials/napi_bindings.rs` and now calls
//! into this module via `codelet_sessions::credentials::*`.

// RPC-040: lifted verbatim from codelet-napi (which does not inherit
// workspace clippy lints) into codelet-sessions (which does). The lints
// relaxed below match the carry-overs already applied to the moved
// `background_session.rs` in RPC-039. Re-tightening is tracked by
// RPC-042.
#![allow(clippy::uninlined_format_args)]

mod resolver;
mod store;
mod types;
mod writer;

pub use resolver::*;
pub use store::*;
pub use types::*;
pub use writer::*;
