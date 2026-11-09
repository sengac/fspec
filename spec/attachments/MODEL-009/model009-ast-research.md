# MODEL-009 AST research (2026-11-04)

## ModelInfo / reasoning fields (rust/providers/src/models)
```
rust/providers/src/models/types.rs:38:1:pub struct ModelInfo {

No matches found
```

## The closed ThinkingLevel enums
```
rust/rpc-types/src/chrome.rs:41:1:pub enum ThinkingLevel {
rust/tools/src/facade/thinking_config.rs:110:1:pub enum ThinkingLevel {

rust/agent-loop/src/thinking_config.rs:24:1:pub enum JsThinkingLevel {

```

## Model lookups that would carry a capability
```
No matches found
```

## Registry entry points (rust/providers/src/models/registry.rs)
```
registry.rs:48   pub fn from_response(response: ModelsDevResponse) -> Self
registry.rs:88   pub fn get_model(&self, provider: &str, model: &str) -> Result<&ModelInfo, ProviderError>
```

## Cache shape (~/.fspec/cache/models.json, 2026-11-04)
```
openai/gpt-5.5            [{"type":"effort","values":["none","low","medium","high","xhigh"]}]
anthropic/claude-opus-4-6 [{"type":"effort","values":["low","medium","high","max"]},{"type":"budget_tokens","min":1024}]
anthropic/claude-haiku-4-5 [{"type":"budget_tokens","min":1024}]
zai/glm-4.5               [{"type":"toggle"}]
google/gemini-2.5-flash   [{"type":"toggle"},{"type":"budget_tokens","min":0,"max":24576}]
```

## Conclusion
ModelInfo has `reasoning: bool` only; `reasoning_options` is dropped by serde. The new
capability lives beside ModelInfo in rust/providers/src/models (a `reasoning` module):
ModelInfo gains `reasoning_options: Option<Vec<ReasoningOption>>`, and
`ReasoningCapability` is built from it per model. The three ThinkingLevel enums are
replaced in the follow-on cards (MODEL-010/011/012), not here.
