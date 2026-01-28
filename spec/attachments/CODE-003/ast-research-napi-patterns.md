# AST Research: NAPI-RS Integration Patterns

## Research Commands Used
```bash
fspec research --tool=ast --pattern="pub use $NAME" --lang=rust --path=codelet/napi/src/
```

## Existing NAPI-RS Structure

### Current Module Organization
- `mod persistence` - Persistence functionality (works in both modes)
- `mod astgrep` - AST grep tool bindings
- `mod glob` - Glob tool bindings  
- `mod models` - Model functionality
- `mod output` - Output handling
- `mod session` - Session management
- `mod session_manager` - Session manager
- `mod thinking_config` - Thinking configuration
- `mod types` - Type definitions

### Export Pattern
All modules are conditionally compiled with `#[cfg(not(feature = "noop"))]` and then re-exported using `pub use module::*;`

```rust
#[cfg(not(feature = "noop"))]
mod astgrep;
#[cfg(not(feature = "noop"))]
pub use astgrep::*;
```

### Key Insights for FspecTool Integration

1. **Follow Existing Pattern**: Create `mod fspec` in napi/src/lib.rs
2. **Conditional Compilation**: Use `#[cfg(not(feature = "noop"))]` wrapper
3. **Re-export Everything**: Use `pub use fspec::*;` to expose all functions
4. **Tool Integration**: Tools seem to be imported and wrapped in specific NAPI modules
5. **ThreadsafeFunction**: May need to use for callbacks as shown in logging module

### Recommended Integration Steps

1. Add `mod fspec;` declaration in napi/src/lib.rs
2. Create `codelet/napi/src/fspec.rs` with NAPI bindings
3. Add `pub use fspec::*;` to re-export FspecTool functions
4. Import FspecTool from `codelet_tools` crate
5. Use napi-derive macros to expose functions to TypeScript

### Pattern Example (from existing code)
```rust
// In codelet/napi/src/lib.rs
#[cfg(not(feature = "noop"))]
mod fspec;
#[cfg(not(feature = "noop"))]
pub use fspec::*;

// In codelet/napi/src/fspec.rs
use codelet_tools::FspecTool;
use napi::bindgen_prelude::*;

#[napi]
pub async fn call_fspec_command(
    command: String,
    args: Option<Vec<String>>,
) -> Result<String> {
    // Implementation
}
```