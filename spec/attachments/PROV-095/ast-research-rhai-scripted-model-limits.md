# AST Research — PROV-095 Rhai-scripted model limits

Scope: understand current construction, accessor, and propagation paths
that PROV-095 must extend so the Rhai script can set `context_window`,
`max_output_tokens`, and `compaction_threshold` at provider construction
time.

## 1. `RhaiCustomProvider::new` (construction site — must call `get_model_limits`)

File: `codelet/providers/src/custom/provider.rs:57-87`

```rust
pub fn new(
    config: Arc<ProviderConfig>,
    loader: Arc<ScriptLoader>,
    model_alias: String,
) -> Result<Self, CustomProviderError> {
    let model_def = config.models.get(&model_alias).ok_or_else(|| {
        CustomProviderError::RhaiRuntimeError(format!(
            "model alias '{model_alias}' not found in config.models"
        ))
    })?;
    let model_id = model_def.id.clone();
    let context_window = model_def.context_window;
    let max_output_tokens = model_def.max_output_tokens;

    let script_path = std::path::PathBuf::from(&config.script);
    let ast = loader.load(&script_path)?;
    loader.validate_required_functions(&ast)?;
    let engine: Arc<Engine> = loader.engine_arc();

    Ok(Self {
        config,
        model_alias,
        model_id,
        context_window,
        max_output_tokens,
        ast,
        engine,
        _loader: loader,
        http_client: reqwest::Client::new(),
    })
}
```

**Key observations:**
- `context_window` and `max_output_tokens` are currently read *only* from
  `model_def` (the JSON ModelDef). There is no Rhai hook.
- After `validate_required_functions(&ast)` but before constructing
  `Self`, the provider must optionally invoke `get_model_limits(config)`.
- The engine & AST are already available at this point — we can call
  `engine.call_fn` synchronously (no tokio::spawn_blocking needed
  because `new` is a non-async function and the Rhai call is cheap).
- Struct must gain a new field, e.g.
  `script_compaction_threshold: Option<(String, u64)>`, populated here.

## 2. Existing `LlmProvider::context_window()` accessor

File: `codelet/providers/src/custom/provider.rs:329-335`

```rust
fn context_window(&self) -> usize {
    self.context_window
}

fn max_output_tokens(&self) -> usize {
    self.max_output_tokens
}
```

**Implication:** Rule 6 of the example map ("resolved value is what
`context_window()`/`max_output_tokens()` return") is satisfied automatically
once `new()` assigns script-overridden values to the struct fields.
No change required in the `LlmProvider` trait impl.

## 3. Rhai call plumbing — call_fn pattern (rhai_call.rs)

File: `codelet/providers/src/custom/rhai_call.rs:12-28`

```rust
pub(crate) async fn call_fn1(
    engine: Arc<Engine>,
    ast: Arc<AST>,
    provider: String,
    fn_name: &'static str,
    arg: Dynamic,
) -> Result<Dynamic, ProviderError> {
    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn::<Dynamic>(&mut scope, &ast, fn_name, (arg,))
            .map_err(|e| map_rhai_error_to_provider(&provider, fn_name, &e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}
```

**Decision for PROV-095:**
`get_model_limits` is called synchronously inside `RhaiCustomProvider::new`
(not via `call_fn1`/`call_fn2`). Reasons:
- `new()` is not async.
- The call happens once per provider lifetime — no need to offload.
- Matches how `validate_required_functions` already invokes the engine
  synchronously from `new`.

For the `FunctionNotFound` case we will use direct `engine.call_fn` and
inspect `rhai::EvalAltResult::ErrorFunctionNotFound(fn_name, ..)` — that
sentinel is treated as "script chose not to define this optional hook"
and we fall through to the JSON defaults with no log warning.

## 4. `ProviderManager::set_compaction_threshold_override`

File: `codelet/providers/src/manager.rs:1078-1080`

```rust
pub fn set_compaction_threshold_override(&mut self, config: Option<(String, u64)>) {
    self.compaction_threshold_override = config;
}
```

**Implication:** The NAPI layer (PROV-095 touch point #3) must call this
with the value returned by a new `RhaiCustomProvider::script_compaction_threshold()`
accessor — only for custom providers whose script returned a valid
`compaction_threshold` entry.

## 5. NAPI session_set_model — existing override wiring

File: `codelet/napi/src/session_manager.rs:6905-6948`

```rust
pub async fn session_set_model(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
    compaction_threshold_type: Option<String>,
    compaction_threshold_value: Option<u32>,
) -> Result<()> {
    // …
    // CTX-008: Set compaction threshold override from TUI configuration
    if let (Some(ct_type), Some(ct_value)) = (&compaction_threshold_type, compaction_threshold_value) {
        inner.provider_manager_mut().set_compaction_threshold_override(
            Some((ct_type.clone(), ct_value as u64))
        );
    } else {
        inner.provider_manager_mut().set_compaction_threshold_override(None);
    }
    // …
    inner.provider_manager_mut().override_model_limits(
        context_window.map(|v| v as usize),
        max_output_tokens.map(|v| v as usize),
    );
}
```

**Implication for PROV-095:**
When the TUI/NAPI caller does NOT pass explicit
`compaction_threshold_type`/`compaction_threshold_value` (the typical
case for a freshly-selected Rhai custom provider), the session-set-model
path should, for custom providers, consult the new
`RhaiCustomProvider::script_compaction_threshold()` accessor and wire
its value into `set_compaction_threshold_override`. TUI-supplied values
still win (existing CTX-008 precedence is preserved).

## 6. Existing `prov_095_*` test file names are TAKEN by unrelated tests

- `tests/prov_095_build_request_iterable_regression_tests.rs` — regression
  for "For loop expects iterable type" bug.
- `tests/prov_095_string_len_regression_tests.rs` — regression for
  `script missing required function 'len(&str)'`.

Both pre-date this story and test a different feature
(`spec/features/custom-provider-script-shadowing-builtin-providers.feature`).

**Decision:** Name the new test file
`tests/rhai_scripted_model_limits_tests.rs` to avoid confusion and
clearly describe what it validates.

## 7. Test-helper reference: `build_provider_inline`

File: `codelet/providers/tests/rhai_rig_agent_keystone_tests.rs:125-168`

This helper already builds a `RhaiCustomProvider` from an inline script
string with a configurable `ModelDef`. The new test file will follow the
same pattern: inline Rhai script source + explicit `ModelDef` so each
scenario is self-contained and requires no fixture files.

## Summary of code change surface

| File | Change |
|------|--------|
| `codelet/providers/src/custom/provider.rs` | Add `get_model_limits` invocation in `new()`, new struct field, new `script_compaction_threshold()` accessor |
| `codelet/providers/src/custom/script_loader.rs` (or new sibling) | Optional helper to parse the `Map` return value with validation (context_window, max_output_tokens, compaction_threshold) |
| `codelet/napi/src/session_manager.rs` | Wire `script_compaction_threshold()` into `session_set_model` as a fallback when TUI did not pass explicit values |
| `codelet/providers/tests/rhai_scripted_model_limits_tests.rs` (new) | 8 scenario tests with @step comments |
