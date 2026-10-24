# Codex routing research

Rust AST query: `impl CodexRoutingHandle { $$$BODY }` in
`rust/providers/src/codex/routing.rs` matched line 41.
The handle retains opaque per-session cache affinity and per-turn routing.
Session ownership prevents sharing between agents; account changes and resets
invalidate routing. The CLI session seam persists routing separately from UI
presentation and begins a new user turn without discarding session affinity.

Tests: prov155_codex_routing and codelet-cli prov155_codex_session_routing.
Initial session integration failure: /tmp/fspec-prov155-session-red.log.
Green provider evidence: /tmp/fspec-codex-completion-providers.log; CLI evidence:
/tmp/fspec-codex-board-cli.log.
