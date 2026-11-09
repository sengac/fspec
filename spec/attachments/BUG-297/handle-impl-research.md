# Research: `handle_impl` — unknown-session error text and stale block_on bound

Target: `cargo test --profile ci-test -p codelet-sessions --test handle_impl`
Result on current tree **and on clean HEAD**: 6 passed, 2 failed. Pre-existing.

## 1. `scenario_unknown_session_id_returns_safe_defaults` (handle_impl.rs:151)
Expectation: for an unknown id (`SessionId::new("nonexistent-uuid")`), every fallible handle
method returns `Err` containing `Session not found`.
Actual for `restore_session_messages`:
```
session restore failed: invalid character: expected an optional prefix of `urn:uuid:`
followed by [0-9a-fA-F-], found `n` at 1
```
`rust/sessions/src/handle_impl.rs:529` now delegates to `self.restore_messages(session_id,…)`
which parses the id as a UUID and surfaces the parse error, instead of using the
`uuid_from(session_id)` → lookup → "Session not found" path the sibling methods use.
Callers and the feature contract rely on the uniform not-found wording.

## 2. `scenario_impl_block_exists_with_every_override` (handle_impl.rs:599)
Source-shape check counts `tokio::runtime::Handle::current().block_on(` occurrences in the
`SessionManagerHandle` impl region and requires `1..=8`; actual **10**. The comment enumerates
the 8 known bridges (resume_session, compact_session, restore_session_messages,
create_isolated_session, test_provider_connection, detach_session_worktree, shared helper,
profile_sections `/v1/models` probe). Two newer bridges were added without updating the
inventory.

## Suggested fix
1. Map invalid/unknown ids in `restore_session_messages` (and `restore_session_token_state`)
   to the same `Session not found` error (e.g. via `uuid_from` + `get_session`), keeping the
   detailed parse error in a `tracing::debug!`.
2. Identify the two new `block_on` sites; either route them through the shared bridge helper
   or update the documented inventory and bound together, so the guard still detects
   accidental new blocking bridges.
