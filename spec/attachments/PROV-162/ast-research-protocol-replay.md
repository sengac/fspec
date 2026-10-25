# AST discovery: Codex protocol replay

Actual AstGrep invocation during native discovery:

- Language: Rust.
- Path: `rust/providers/src/codex/lifecycle.rs`.
- Complete-node pattern:
  `pub fn replay(&self, request: &mut Value) -> Result<(), CompletionError> { $$$BODY }`
- Actual result: `rust/providers/src/codex/lifecycle.rs:123:5`, public method
  `replay` on `Terminal`.
- The first pattern omitted `pub` and found no matches; it is not represented
  as a successful structural search.

Bounded source review shows this method rejects empty or non-text output, then
appends directly to `request.input`. The real caller is the eligible continuation
branch of `codex/streaming.rs`, after physical usage settlement and before the
next full serialized request admission. Existing `CodexItem` represents reasoning,
messages and negotiated hosted search; native and memory-policy checks remain
required. Existing history/model/ancestry and transport account/generation gates
must remain attached. This discovery supports the native-reviewed narrow replay
validator, not a new registry, delta protocol or general native-field replay.
