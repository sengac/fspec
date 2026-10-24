# Codex request options discovery

AST query (Rust): `fn build_reasoning_params($$$ARGS) -> $RETURN { $$$BODY }`
Scope: `rust/providers/src/codex`.
Result: `mod.rs:123`, `CodexProvider::build_reasoning_params`.

The helper currently injects high effort and automatic summary independent of
capabilities and only forwards reasoning from thinking configuration. The agent
calls this helper before dispatch. The parent owns replacing that call so explicit
text, service tier and stream options survive until Codex model resolution.

The Codex model currently wraps rig Responses conversion, whose
AdditionalParameters conversion expects successful deserialization. Codex-only
options must be validated before conversion and kept outside shared effort/text
enums. Resolver tests cover named/numeric effort, optional gates, fallback,
precedence and arbitrary JSON. The local HTTP isolation test remains red until
the parent's model transport seam integrates the resolver.

Upstream reference bb64b89, codex-rs/codex-api/src/common.rs confirms numeric
custom effort serialization, context auto/current_turn/all_turns, independent
text verbosity and stream_options.reasoning_summary_delivery=sequential_cutoff.
