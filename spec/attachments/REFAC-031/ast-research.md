# AST research: REFAC-031

Runs over the original file (HEAD copy of rust/providers/src/openai.rs, 631 lines) and over the split module tree.

## `impl $T for OpenAIProvider { $$$B }` in HEAD openai.rs
```
openai.rs(HEAD):115:1:impl std::fmt::Debug for OpenAIProvider {
openai.rs(HEAD):126:1:impl ProviderAdapter for OpenAIProvider {
openai.rs(HEAD):136:1:impl crate::model_limits::ModelLimitsResolver for OpenAIProvider {
openai.rs(HEAD):509:1:impl LlmProvider for OpenAIProvider {

```

## `impl OpenAIProvider { $$$B }` in HEAD openai.rs
```
openai.rs(HEAD):160:1:impl OpenAIProvider {

```

## `fn $F($$$A) -> $R { $$$B }` in HEAD openai.rs
```
openai.rs(HEAD):59:1:fn anti_ramble_additional_params() -> serde_json::Value {
openai.rs(HEAD):83:1:fn normalize_base_url(url: &str) -> String {
openai.rs(HEAD):116:5:fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
openai.rs(HEAD):127:5:fn provider_name(&self) -> &'static str {
openai.rs(HEAD):138:5:fn max_context_window(&self) -> Option<usize> {
openai.rs(HEAD):143:5:fn max_output_tokens_limit(&self) -> Option<usize> {
openai.rs(HEAD):149:5:fn default_context_window(&self) -> Option<usize> {
openai.rs(HEAD):155:5:fn default_max_output_tokens(&self) -> Option<usize> {
openai.rs(HEAD):477:5:fn rig_response_to_completion(
openai.rs(HEAD):510:5:fn name(&self) -> &str {
openai.rs(HEAD):514:5:fn model(&self) -> &str {
openai.rs(HEAD):518:5:fn context_window(&self) -> Option<usize> {
openai.rs(HEAD):522:5:fn max_output_tokens(&self) -> Option<usize> {
openai.rs(HEAD):526:5:fn supports_caching(&self) -> bool {
openai.rs(HEAD):530:5:fn supports_streaming(&self) -> bool {

```

## `const $C: $T = $V;` in HEAD openai.rs
```
openai.rs(HEAD):27:1:const DEFAULT_CONTEXT_WINDOW: usize = 128_000;
openai.rs(HEAD):31:1:const DEFAULT_MAX_OUTPUT_TOKENS: usize = 4096;
openai.rs(HEAD):51:1:const ANTI_RAMBLE_TEMPERATURE: f64 = 0.7;
openai.rs(HEAD):52:1:const ANTI_RAMBLE_TOP_P: f64 = 0.8;
openai.rs(HEAD):53:1:const ANTI_RAMBLE_TOP_K: u32 = 20;
openai.rs(HEAD):54:1:const ANTI_RAMBLE_MIN_P: f64 = 0.0;
openai.rs(HEAD):55:1:const ANTI_RAMBLE_PRESENCE_PENALTY: f64 = 1.5;
openai.rs(HEAD):56:1:const ANTI_RAMBLE_REPETITION_PENALTY: f64 = 1.0;

```

## `mod $M;` in HEAD openai.rs
```
openai.rs(HEAD):23:1:mod discovery;

```

## `impl $T for OpenAIProvider { $$$B }` in rust/providers/src/openai (after split)
```
rust/providers/src/openai.rs:111:1:impl std::fmt::Debug for OpenAIProvider {
rust/providers/src/openai.rs:122:1:impl ProviderAdapter for OpenAIProvider {

rust/providers/src/openai/llm_provider.rs:20:1:impl crate::model_limits::ModelLimitsResolver for OpenAIProvider {
rust/providers/src/openai/llm_provider.rs:78:1:impl LlmProvider for OpenAIProvider {

```

## `impl OpenAIProvider { $$$B }` in rust/providers/src/openai (after split)
```
No matches found
rust/providers/src/openai/construction.rs:10:1:impl OpenAIProvider {
rust/providers/src/openai/llm_provider.rs:44:1:impl OpenAIProvider {
rust/providers/src/openai/agent.rs:10:1:impl OpenAIProvider {

```

## `mod $M;` in rust/providers/src/openai (after split)
```
rust/providers/src/openai.rs:13:1:mod discovery;
rust/providers/src/openai.rs:15:1:mod agent;
rust/providers/src/openai.rs:16:1:mod construction;
rust/providers/src/openai.rs:17:1:mod llm_provider;
rust/providers/src/openai.rs:19:1:mod tests;

No matches found
```

## `openai::$X` uses outside the module (rust/providers/src/manager)
```
rust/providers/src/manager/impl_4.rs:158:32:openai::CONTEXT_WINDOW
rust/providers/src/manager/impl_4.rs:162:32:openai::MAX_OUTPUT_TOKENS

```
