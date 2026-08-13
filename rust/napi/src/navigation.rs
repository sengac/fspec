//! Session navigation re-exports (RPC-040).
//!
//! Lifted to `codelet-sessions` by RPC-040. This module remains as a
//! thin re-export shim so callers that still import
//! `crate::navigation::*` continue to compile unchanged.

pub use codelet_sessions::navigation::{
    build_navigation_list, get_next_target, get_prev_target, NavigationTarget,
};
