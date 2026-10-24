# Metadata integration research

AstGrep Rust `pub fn handle_output($$$ARGS) { $$$BODY }` finds
`rust/sessions/src/background_session.rs:984`. This is the existing fanout to
session buffer, supervisor broadcast and UI/RPC chunks. Emit only existing
UserNotification from a weak-session safe metadata observer.

Transport was audited in `rust/providers/src/codex/streaming.rs` and pinned
upstream `/tmp/codex/codex-rs/codex-api/src/sse/responses.rs` at bb64b89.
The GenericEventSource AST search returned no matches while worker148 was
editing that file; textual inspection originally showed header-dropping source.
Capture HTTP response headers before SSE conversion using HttpClientExt.

Session dispatch presently selects Codex via run_with_provider(get_codex).
Coordinate a Codex-only setup hook with149 rather than touching generic parsing.
Design details and proposed integration API are in design-api.md.
