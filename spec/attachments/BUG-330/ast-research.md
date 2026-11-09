# AST research: BUG-330

## `fn default_context_window(&self) -> usize { $$$B }` in rust/providers/src
```
rust/providers/src/gemini.rs:59:5:fn default_context_window(&self) -> usize {
rust/providers/src/zai.rs:80:5:fn default_context_window(&self) -> usize {
rust/providers/src/claude.rs:196:5:fn default_context_window(&self) -> usize {
rust/providers/src/copilot/provider.rs:225:5:fn default_context_window(&self) -> usize {
rust/providers/src/model_limits.rs:201:9:fn default_context_window(&self) -> usize {
rust/providers/src/model_limits.rs:357:13:fn default_context_window(&self) -> usize {
rust/providers/src/model_limits.rs:372:13:fn default_context_window(&self) -> usize {
rust/providers/src/model_limits.rs:387:13:fn default_context_window(&self) -> usize {
rust/providers/src/codex/mod.rs:128:5:fn default_context_window(&self) -> usize {
rust/providers/src/manager.rs:191:5:fn default_context_window(&self) -> usize {
rust/providers/src/openai.rs:149:5:fn default_context_window(&self) -> usize {

```

## `fn default_max_output_tokens(&self) -> usize { $$$B }` in rust/providers/src
```
rust/providers/src/gemini.rs:64:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/zai.rs:85:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/claude.rs:201:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/copilot/provider.rs:230:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/model_limits.rs:205:9:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/model_limits.rs:360:13:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/model_limits.rs:375:13:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/model_limits.rs:390:13:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/codex/mod.rs:133:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/manager.rs:194:5:fn default_max_output_tokens(&self) -> usize {
rust/providers/src/openai.rs:155:5:fn default_max_output_tokens(&self) -> usize {

```

## `pub const CONTEXT_WINDOW: usize = $V;` in rust/providers/src
```
rust/providers/src/gemini.rs:24:1:pub const CONTEXT_WINDOW: usize = 1_000_000;
rust/providers/src/zai.rs:34:1:pub const CONTEXT_WINDOW: usize = 128_000;
rust/providers/src/claude.rs:46:1:pub const CONTEXT_WINDOW: usize = 200_000;
rust/providers/src/copilot/mod.rs:78:1:pub const CONTEXT_WINDOW: usize = 200_000;
rust/providers/src/codex/mod.rs:60:1:pub const CONTEXT_WINDOW: usize = 272_000;
rust/providers/src/openai.rs:70:1:pub const CONTEXT_WINDOW: usize = DEFAULT_CONTEXT_WINDOW;

```

## `pub const MAX_OUTPUT_TOKENS: usize = $V;` in rust/providers/src
```
rust/providers/src/gemini.rs:27:1:pub const MAX_OUTPUT_TOKENS: usize = 8192;
rust/providers/src/zai.rs:37:1:pub const MAX_OUTPUT_TOKENS: usize = 8192;
rust/providers/src/claude.rs:49:1:pub const MAX_OUTPUT_TOKENS: usize = 8192;
rust/providers/src/copilot/mod.rs:86:1:pub const MAX_OUTPUT_TOKENS: usize = 4_096;
rust/providers/src/codex/mod.rs:63:1:pub const MAX_OUTPUT_TOKENS: usize = 4096;
rust/providers/src/openai.rs:73:1:pub const MAX_OUTPUT_TOKENS: usize = DEFAULT_MAX_OUTPUT_TOKENS;

```

## `fn context_window(&self) -> usize { $$$B }` in rust/providers/src/codex
```
rust/providers/src/codex/completion.rs:62:5:fn context_window(&self) -> usize {

```

## `codex::CONTEXT_WINDOW` in rust/providers/src
```
No matches found
```

## `ProviderType::Codex => Box::new(ConstantResolver { $$$F })` in rust/providers/src/manager.rs
```
No matches found
```

## `fn codex_falls_back_to_272k_default() { $$$B }` in rust/providers/src
```
rust/providers/src/model_limits.rs:785:5:fn codex_falls_back_to_272k_default() {

```

## `fn codex_resolver_returns_correct_defaults_and_suppresses_max_output_tokens() { $$$B }` in rust/providers/src
```
rust/providers/src/model_limits.rs:588:5:fn codex_resolver_returns_correct_defaults_and_suppresses_max_output_tokens() {

```
