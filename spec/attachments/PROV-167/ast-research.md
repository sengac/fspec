# PROV-167 AST research

## fspec astgrep --pattern 'fn context_window(&self) -> usize { $$$B }' --lang rust --path rust/providers/src
rust/providers/src/gemini.rs:337:5:fn context_window(&self) -> usize {
rust/providers/src/zai.rs:366:5:fn context_window(&self) -> usize {
rust/providers/src/stub_provider.rs:109:5:fn context_window(&self) -> usize {
rust/providers/src/claude.rs:700:5:fn context_window(&self) -> usize {
rust/providers/src/copilot/provider.rs:245:5:fn context_window(&self) -> usize {
rust/providers/src/codex/completion.rs:65:5:fn context_window(&self) -> usize {
rust/providers/src/custom/provider.rs:228:5:fn context_window(&self) -> usize {
rust/providers/src/openai.rs:518:5:fn context_window(&self) -> usize {


## fspec astgrep --pattern 'fn max_output_tokens(&self) -> usize { $$$B }' --lang rust --path rust/providers/src
rust/providers/src/gemini.rs:341:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/zai.rs:370:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/stub_provider.rs:113:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/claude.rs:708:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/copilot/provider.rs:252:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/codex/completion.rs:69:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/custom/provider.rs:232:5:fn max_output_tokens(&self) -> usize {
rust/providers/src/openai.rs:522:5:fn max_output_tokens(&self) -> usize {


## fspec astgrep --pattern 'LlmProvider::context_window($$$A)' --lang rust --path rust/providers/tests
rust/providers/tests/bug330_codex_static_limits.rs:53:9:LlmProvider::context_window(&provider)
rust/providers/tests/bug330_codex_static_limits.rs:69:9:LlmProvider::context_window(&provider)


## fspec astgrep --pattern 'provider.max_output_tokens()' --lang rust --path rust/sessions/tests
No matches found

## fspec astgrep --pattern 'provider.context_window()' --lang rust --path rust/core/tests
No matches found

## Callers found with grep (method-call patterns on other receivers)

- `rust/providers/src/openai.rs:609-629` (unit tests), `rust/core/tests/rig_provider_streaming_test.rs:222-223`,
  `rust/sessions/tests/cmpct083/zai.rs:26-29` (`provider.max_output_tokens()` on a Z.AI provider).
- No production code calls `LlmProvider::context_window` / `max_output_tokens`.
