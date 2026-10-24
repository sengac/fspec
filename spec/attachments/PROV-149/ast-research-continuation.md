# Typed continuation integration research

AstGrep (Rust) located `CodexHistory::prepare` at
`rust/providers/src/codex/history.rs:74` using the complete function pattern
`pub fn prepare(&self, request: &mut Value) -> Result<(), CompletionError> { $$$BODY }`.

The Codex model calls `prepare` after serializing ordinary history and commits
typed output only after successful terminal validation. The history maintains
opaque items separately from readable frontend thinking. User-prefix and model
checks invalidate stale continuation rather than combining unrelated histories.

Backend integration is exercised through `SessionManager`, manifest envelopes,
blob rehydration, and `bind_codex_persistence`, not a serializer-only simulation.
Session clear and local compaction invalidate the same handle. Legacy envelopes
without optional continuation metadata retain ordinary provider behavior.

Relevant tests: `prov149_codex_history` (ordered parallel tool results and
property-based opaque round trips), and agent-loop `prov149_codex_reload`
(backend persistence, restoration, legacy behavior, replacement).
