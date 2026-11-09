# MODEL-011 AST research (2026-11-05)

## The wire ThinkingLevel enum (rpc-types)
```
rust/rpc-types/src/chrome.rs:41:1:pub enum ThinkingLevel {

```

## Session storage of the level (u8)
```
rust/sessions/src/background_session/metrics.rs:70:5:pub fn set_base_thinking_level(&self, level: u8) {

```

## Conversions from the stored u8
```
rust/agent-loop/src/thinking_level_detection.rs:14:1:pub fn base_thinking_level_from_u8(value: u8) -> Result<Option<TurnLevel>, String> {

```

## Handle setters and getters
```
No matches found
```

## The thinking target the turn builds
```
rust/agent-loop/src/thinking_config.rs:154:5:pub fn for_provider_manager(manager: &ProviderManager) -> Self {

```

## The dialog's fixed level table
```
rust/fspec-tui/src/components/thinking_level_dialog.rs:26:1:const LEVELS: [(ThinkingLevel, &str, &str); 5] = [

```

## /thinking argument parsing
```
rust/fspec-tui/src/app/slash_parser.rs:127:9:let level = match arg.as_str() {

```

## Header thinking label
```
rust/fspec-tui/src/views/agent/header_build.rs:184:1:fn thinking_label(level: ThinkingLevel) -> Option<&'static str> {

```

## Model limits cache (pattern for the thinking target cache)
```
rust/sessions/src/background_session/metrics.rs:49:5:pub fn set_model_limits(&self, limits: crate::model_resolution::ResolvedModelLimits) {

```

