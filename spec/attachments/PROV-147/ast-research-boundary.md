# Boundary research

AstGrep Rust pattern `impl ProviderAdapter for $TYPE { $$$BODY }` scoped to
`rust/providers/src/codex` finds `CodexProvider` in `mod.rs:77`.
Its provider identity is explicitly `codex`.

Inspection of the construction and agent builder shows that direct completions
use the `CodexResponsesModel` alias, but agent construction independently calls
`rig_client.agent`. Both must select the composed model to establish the boundary.
The existing OpenAI provider uses its own Chat Completions model and builder;
neither it nor shared rig internals needs modification.
