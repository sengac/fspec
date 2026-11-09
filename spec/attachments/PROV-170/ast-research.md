# PROV-170 AST research

Runs of `fspec astgrep --lang rust` with narrow paths. Raw output:

```
### pattern: `$X.context_window` path: rust/providers/src/custom
rust/providers/src/custom/provider.rs:104:35:model_def.context_window
rust/providers/src/custom/provider.rs:120:42:overrides.context_window
rust/providers/src/custom/provider.rs:122:30:overrides.context_window
rust/providers/src/custom/provider.rs:173:18:self.context_window
rust/providers/src/custom/provider.rs:229:14:self.context_window
rust/providers/src/custom/management.rs:200:33:def.context_window

### pattern: `$X.max_output_tokens` path: rust/providers/src/custom
rust/providers/src/custom/tool_dispatch_extras.rs:109:28:s.max_output_tokens
rust/providers/src/custom/provider.rs:105:38:model_def.max_output_tokens
rust/providers/src/custom/provider.rs:121:45:overrides.max_output_tokens
rust/providers/src/custom/provider.rs:123:33:overrides
rust/providers/src/custom/provider.rs:184:18:self.max_output_tokens
rust/providers/src/custom/provider.rs:233:14:self.max_output_tokens
rust/providers/src/custom/management.rs:201:36:def.max_output_tokens

### pattern: `u32::try_from($M.context_window).unwrap_or(u32::MAX)` path: rust/sessions/src/handle_impl
No matches found
### pattern: `default_context_window()` path: rust/providers/src/custom
No matches found
### pattern: `default_max_output_tokens()` path: rust/providers/src/custom
No matches found
### pattern: `u32::try_from($V)` path: rust/sessions/src/handle_impl/handle_impl_5.rs
No matches found
### pattern: `u32::try_from($V)` path: rust/sessions/src/handle_impl/handle_impl_8.rs
No matches found
### pattern: `fn $F() -> usize { $$$B }` path: rust/providers/src/custom/config.rs
rust/providers/src/custom/config.rs:43:1:fn default_context_window() -> usize {
rust/providers/src/custom/config.rs:48:1:fn default_max_output_tokens() -> usize {

```

## Findings

- The only producers of the guess are the two `fn ... -> usize` serde default
  functions in `rust/providers/src/custom/config.rs:43` and `:48`, referenced as
  string attributes (`#[serde(default = "...")]`), which is why the call-pattern
  queries find nothing.
- Readers of `ModelDef.context_window` / `max_output_tokens` inside the custom
  module: `provider.rs:104-105` (RhaiCustomProvider::new) and
  `management.rs:200-201` (list_providers_info). `tool_dispatch_extras.rs:109`
  is an unrelated tool-dispatch struct.
- `rust/sessions/src/handle_impl/handle_impl_5.rs` and `handle_impl_8.rs` are
  `macro_rules!` bodies, so ast-grep cannot parse the `u32::try_from(m.context_window)`
  sites inside them (no matches); they were located with grep
  (`handle_impl_5.rs:61-63`, `handle_impl_8.rs:50`).
- The ProviderManager / agent path does not read `ModelDef` at all:
  `rust/providers/src/selection_metadata.rs:70-72` reads the raw JSON
  (`context_window`, `max_output_tokens` or `defaults.max_output_tokens`) as
  `Option`s, and `manager_runtime.rs:17-19` overlays the script-only limits
  (`script_context_window` / `script_max_output_tokens`), which are already `Option`.
