# BUG-547 AST research: where a profile model's limits are resolved and refused

## `apply_model_selection($$$A)` in rust/sessions/src
```
rust/sessions/src/model_resolution.rs:275:18:apply_model_selection(&mut provider_manager, model)
```

## `resolve_current_model_limits($M)` in rust/sessions/src
```
rust/sessions/src/model_resolution.rs:239:5:resolve_current_model_limits(pm)
rust/sessions/src/runtime_settings.rs:22:13:resolve_current_model_limits(manager)
```

## `fn get_openai($$$A) -> $R { $$$ }` in rust/providers/src/manager
```
No matches found
```

## `set_model_direct_with_profile($$$A)` in rust/sessions/src
```
No matches found
```

## `LiveCheck::for_model($$$A)` in rust
```
No matches found
```

## `save_profile_at($$$A)` in rust/sessions/src
```
rust/sessions/src/profile_persistence.rs:119:5:save_profile_at(&config_path, provider_id, profile_name, def)
rust/sessions/src/profile_persistence.rs:360:16:save_profile_at(config_path, provider_id, new_name, def)
rust/sessions/src/profile_persistence_tests.rs:46:5:save_profile_at(&path, "openai", "work-vllm", &basic("http://h", "k"))
rust/sessions/src/profile_persistence_tests.rs:63:5:save_profile_at(&path, "openai", "work-vllm", &basic("http://new", "new"))
rust/sessions/src/profile_persistence_tests.rs:81:18:save_profile_at(&path, "anthropic", "work-vllm", &basic("http://x", "y"))
rust/sessions/src/profile_persistence_tests.rs:120:5:save_profile_at(&path, "openai", "work-vllm", &def)
```

## `Action::EmitSessionNotice($$$A)` in rust/fspec-tui/src/app
```
rust/fspec-tui/src/app/dispatch_slash_update.rs:89:36:Action::EmitSessionNotice(session_id, message)
rust/fspec-tui/src/app/dispatch_supervisor_links.rs:61:44:Action::EmitSessionNotice(
rust/fspec-tui/src/app/dispatch_model_sync.rs:61:44:Action::EmitSessionNotice(
rust/fspec-tui/src/app/dispatch_slash_loop.rs:42:19:Action::EmitSessionNotice(session_id, format_loop_help())
rust/fspec-tui/src/app/dispatch_slash_loop.rs:78:36:Action::EmitSessionNotice(sid_for_task, text)
rust/fspec-tui/src/app/dispatch_slash_loop.rs:98:36:Action::EmitSessionNotice(sid_for_task, text)
rust/fspec-tui/src/app/dispatch_slash_loop.rs:119:36:Action::EmitSessionNotice(sid_for_task, text)
rust/fspec-tui/src/app/dispatch_committed_compaction.rs:60:19:Action::EmitSessionNotice(session.clone(), text)
rust/fspec-tui/src/app/dispatch_slash_debug.rs:81:36:Action::EmitSessionNotice(session_for_send, text)
rust/fspec-tui/src/app/dispatch_slash_commands.rs:91:48:Action::EmitSessionNotice(
rust/fspec-tui/src/app/dispatch_continue_sync.rs:123:40:Action::EmitSessionNotice(
rust/fspec-tui/src/app/session_creation_tasks.rs:74:37:Action::EmitSessionNotice(
```

## Findings
- apply_model_selection (sessions/src/model_resolution.rs) routes profile models to profile_model_limits::apply_profile_model and returns resolve_current_model_limits; None limits are stored and nothing refuses them (PROV-165 refusal is only in ProviderManager::get_openai at the turn).
- resolve_provider_manager_and_limits is the async step for create, resume and set_model: the engine query and the refusal go there, after apply_model_selection.
- set_model_direct_with_profile resets user limits on every selection, and session creation applies the selection twice, so an engine-reported value must live on the manager outside the per-selection reset (like MODEL-014's codex catalog).
- LiveCheck (MODEL-016) already reaches llama.cpp /props, Ollama /api/show and LM Studio /api/v1/models from the profile base URL.
- TUI: model-selection and session-creation failures surface as EmitSessionNotice; the prompt hooks in there.
