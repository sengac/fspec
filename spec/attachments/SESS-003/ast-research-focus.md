# SESS-003 AST research

AstGrep (Rust), `rust/sessions/src/background_session.rs`, pattern
`impl BackgroundSession { $$$BODY }` found the implementation at line 512.
The existing context setter is inside this implementation at line 823 and uses
`inner.try_lock()` while authorization reads the same optional context.

Additional inspected call graph: `handle_impl::set_work_unit_context` delegates to
that setter; `session_tool_callbacks::work_unit_stage` reads its context;
`SessionManager::create_session_from_manifest` constructs runtime state without
restoring work focus. `manifest_updates::update_session_manifest` is the serialized
writer boundary to preserve concurrent messages while updating focus.

Decision: introduce independent revisioned activity focus and retain authorization
context when activity is cleared. Use deferred reminder reconciliation, publish
focus events via the owning manager and hydrate snapshots from session metadata.
