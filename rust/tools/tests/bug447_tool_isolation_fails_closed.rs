//! BUG-447 — tool isolation fails closed without an isolation callback.
//! This binary never registers a callback nor declares unrestricted access.
//!
//! Feature: spec/features/tool-isolation-fails-closed-without-a-callback.feature

use codelet_tools::facade::get_isolation_context;
use uuid::Uuid;

#[test]
fn a_sessions_isolation_lookup_fails_without_a_callback_or_declaration() {
    // @step Given a process with no isolation callback and no unrestricted-access declaration
    // @step When the isolation context of a session is looked up
    let lookup = get_isolation_context(Uuid::new_v4());
    // @step Then the lookup fails saying no isolation callback is registered
    let error = lookup.err().unwrap_or_default();
    assert!(
        error.contains("no isolation callback is registered"),
        "{error}"
    );
}

#[test]
fn the_explicit_no_session_id_stays_unrestricted() {
    // @step Given a process with no isolation callback and no unrestricted-access declaration
    // @step When the isolation context of the nil session id is looked up
    let lookup = get_isolation_context(Uuid::nil());
    // @step Then no isolation applies
    assert!(matches!(lookup, Ok(None)), "{lookup:?}");
}
