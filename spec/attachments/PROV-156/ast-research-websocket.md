# Codex WebSocket research

Rust AST query: `impl Baseline { $$$BODY }` in
`rust/providers/src/codex/websocket_baseline.rs` matched line 26.
Only successful responses establish a reusable baseline. Request properties
must match, and typed history must start with the previous request plus output.
Normalizing both sides through CodexItem resolves server-output versus typed
replay differences without loosening equality. Invalid data disables reuse.

Initial failing evidence: /tmp/fspec-prov156-red.log. Stronger parity regression
failed in /tmp/fspec-codex-websocket-parity.log before normalization; it passes
in /tmp/fspec-codex-websocket-green.log. Tests also verify fresh connections
after cancellation, bounded retirement, accepted disconnect errors, HTTP fallback
before acceptance only, and tool/reasoning/text/usage parity.
