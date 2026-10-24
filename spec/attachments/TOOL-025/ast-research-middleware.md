# TOOL-025 AST boundary research

AstGrep Rust query executed against `rust/patches/rig-core/src/tool`:

```rust
pub async fn call(&self, $NAME: &str, args: String) -> Result<String, ToolSetError> { $$$BODY }
```

Result: `rust/patches/rig-core/src/tool/mod.rs:494`, ToolSet::call.
This boundary dispatches the underlying ToolType before typed ToolDyn argument
deserialization. ToolType includes both simple and embedding implementations.
ToolServer has separate static and vector-selected definition paths; both must
retrieve definitions through a shared fallible middleware method rather than
calling raw ToolType::definition directly. Keep application session dependencies
out of Rig and inject immutable-scope middleware with acknowledged configuration.

Provider direct completion APIs do not execute through ToolServer; their raw
definitions require explicit decoration at provider conversion. Custom scripts
require validation after request construction, and Codex raw/hosted registrations
must not discard the JSON attribution contract.
