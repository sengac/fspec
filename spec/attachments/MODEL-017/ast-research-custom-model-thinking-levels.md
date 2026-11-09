# MODEL-017 AST research: custom-provider models and the thinking target

## `ProviderType::Custom($N)` in rust/providers/src/thinking_wire.rs
```
No matches found
```

## `fn selection_metadata($$$A) -> $R { $$$ }` in rust/providers/src/selection_metadata.rs
```
No matches found
```

## `ThinkingWire::Rhai => $E` in rust/providers/src/thinking_wire
```
No matches found
```

## `rhai_config($$$A)` in rust/providers/src/thinking_wire
```
rust/providers/src/thinking_wire/forms.rs:45:43:rhai_config(json!("off"), capability)
rust/providers/src/thinking_wire/forms.rs:59:20:rhai_config(json!(name.as_str()), capability)
rust/providers/src/thinking_wire/forms.rs:62:20:rhai_config(json!(budget), capability)
rust/providers/src/thinking_wire/forms.rs:65:20:rhai_config(json!(number), capability)
rust/providers/src/thinking_wire/forms.rs:67:60:rhai_config(json!("on"), capability)
```

## `fn resolve(&self, $$$A) -> $R { $$$ }` in rust/providers/src/local_reasoning
```
No matches found
```

## `request.thinking_config` in rust/providers/templates/codex
```
No matches found
```

## Findings
- ThinkingTarget::for_provider_manager gives ProviderType::Custom models capability Ok(None) with the Advertised source, so config_for sends any level unchecked (MODEL-010) while the offer says no reasoning control (MODEL-011).
- selection_metadata reads the raw custom config and resolves the model alias by key, then by a unique id; the same lookup can carry the model's raw thinkingLevels.
- The Rhai wire passes {level, capability} through rhai_config; Off always sends level off.
- ThinkingLevelsDeclaration::resolve validates parts against a LocalEngine; a Rhai script owns its wire, so an engine-free resolution is needed.
- codex.rhai reads request.thinking_config.level (a string) and maps off to none.
