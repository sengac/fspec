# Codex native output research

Rust AST query: `impl NativeOutput { $$$BODY }` in
`rust/providers/src/codex/native.rs` matched line 37.
NativeOutput requires explicit capability opt-in for hosted search and opaque
compaction. Hosted calls never enter the local dispatcher. History commits only
at successful terminal boundaries; opaque compaction replaces replay history
while preserving a separate user-input baseline. Optional metadata is ignored;
unknown executable output fails closed. Deferred families are enumerated in the
feature architecture notes rather than advertised as supported.

Initial failures: /tmp/fspec-prov158-options-red.log and
/tmp/fspec-prov158-hosted-red.log. Transport tests additionally verify actual
hosted presentation/replay, serialized checkpoint reload, failed compaction,
local reset, disabled capabilities and ordinary-provider completion. Green
evidence is recorded in /tmp/fspec-codex-completion-providers.log.
