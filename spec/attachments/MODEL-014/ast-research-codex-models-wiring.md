# MODEL-014 AST research: where the Codex /models catalog is wired

## `ThinkingTarget::for_provider_manager($M)` in rust
```
rust/agent-loop/src/agent_loop/routing.rs:20:27:ThinkingTarget::for_provider_manager(inner.provider_manager())
rust/agent-loop/tests/model011_session_holds_the_chosen_thinking_level.rs:67:9:ThinkingTarget::for_provider_manager(inner.provider_manager())
rust/agent-loop/tests/model016_support/mod.rs:127:8:ThinkingTarget::for_provider_manager(&select(
```

## `$M.get_codex()` in rust (non-test)
```
rust/cli/src/interactive_helpers.rs:644:28:manager.get_codex()
rust/providers/src/codex/manager_admission_tests.rs:119:21:manager.get_codex()
rust/agent-loop/src/deep_search_handler.rs:641:28:manager
```

## `resolve_provider_manager_and_limits($M)` in rust/sessions/src
```
rust/sessions/src/model_resolution.rs:264:5:resolve_provider_manager_and_limits(model)
```

## `apply_model_selection($$$A)` in rust/sessions/src
```
rust/sessions/src/model_resolution.rs:275:18:apply_model_selection(&mut provider_manager, model)
```

## `$P.set_capabilities($C)` in rust
```
rust/providers/src/codex/request_options.rs:12:9:self.completion_model.set_capabilities(capabilities)
rust/providers/tests/prov152_codex_provider_capabilities.rs:36:5:provider.set_capabilities(Some(serde_json::from_value::<CodexCapabilities>(
rust/providers/tests/prov152_codex_provider_capabilities.rs:46:5:provider.set_capabilities(Some(serde_json::from_value::<CodexCapabilities>(json!({
rust/providers/tests/prov152_codex_provider_capabilities.rs:56:5:provider.set_capabilities(None)
rust/providers/tests/prov152_codex_capability_refresh.rs:66:5:model.set_capabilities(Some(without_xhigh))
rust/providers/tests/prov152_codex_capability_refresh.rs:84:5:model.set_capabilities(Some(capabilities.clone()))
rust/providers/tests/prov152_codex_capability_refresh.rs:119:5:model.set_capabilities(Some(CodexCapabilities {
```

## `select(&mut $O, $$$A)` in rust/providers/src/codex
```
rust/providers/src/codex/request_options.rs:34:5:select(
rust/providers/src/codex/request_options.rs:46:5:select(
rust/providers/src/codex/request_options.rs:58:5:select(
rust/providers/src/codex/request_options.rs:74:5:select(
rust/providers/src/codex/request_options.rs:96:5:select(
rust/providers/src/codex/request_options.rs:105:5:select(
```

## Findings
- Every Codex provider is built by ProviderManager::get_codex (turns, compaction, deep search, CLI), so the provider snapshot is set there.
- ThinkingTarget::for_provider_manager is built at session creation, set_model and every turn (runtime_settings) from the session's ProviderManager; the catalog stored on that manager reaches all three without refetching.
- resolve_provider_manager_and_limits is the one async model-resolution step for create, resume (resolve_provider_manager) and set_model (block_in_place); the /models fetch goes there after apply_model_selection.
- set_capabilities has callers only in tests before this card.
- request_options::select treats Some(empty list) as nothing supported; the snapshot controls become Option so None means not advertised.
