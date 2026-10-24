# Codex dispatch research

Rust AST query: `impl OutputState { $$$BODY }` in
`rust/providers/src/codex/stream_events.rs` matched line 31.
OutputState reconciles item completion before terminal delivery. Dispatch
identity belongs to tool_dispatch, while presentation receives incomplete input
without authorizing execution. Existing rig hooks remain the permission boundary.

Tests: prov151_codex_dispatch and prov151_codex_presentation. Initial failing
suite evidence: /tmp/fspec-codex-board-red.log. Current test evidence is recorded
in /tmp/fspec-codex-completion-providers.log. Feature splits preserve one test
file per feature, with individual scenario ranges in coverage sidecars.
