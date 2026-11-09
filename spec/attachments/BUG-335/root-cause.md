# BUG-335 root cause: a partial model name borrows another model's catalog capacity

## Symptom

`cargo test --profile ci-test -p codelet-sessions --test codex_catalog_limit_resolution`
fails in `a_partial_codex_model_name_does_not_authorize_catalog_capacity`: after
`set_model("codex", "gpt-4")` the provider reports `context_window() == Some(128000)`
where the scenario requires unavailable (`None`).

## Evidence

- The fixture `rust/sessions/tests/fixtures/prov129_codex_models.json` lists `gpt-4o`
  (context 128000, output 16384) and no `gpt-4`.
- `ProviderManager::refresh_runtime_options` (rust/providers/src/manager_runtime.rs:65-96)
  takes the limits from `selection_model_info` (manager_runtime.rs:98-113), which calls
  `ModelRegistry::get_model("openai", "gpt-4")`.
- `get_model` (rust/providers/src/models/registry.rs:88-105) delegates to
  `lookup::find_model` (rust/providers/src/models/lookup.rs:4-20). After the exact key and
  exact id checks fail, its third branch accepts any *unique* entry whose key or id
  `starts_with(model)`: `"gpt-4o".starts_with("gpt-4")`, so `gpt-4` resolves to `gpt-4o`
  and inherits 128000/16384.

## Pre-existing, not caused by PROV-165

- `lookup.rs` is unchanged since 2699693b (`git log -- rust/providers/src/models/lookup.rs`).
- Before PROV-165 (3ef07f2c) the same test asserted `context: 0`; the lookup returned
  128000 then too, so the test failed before PROV-165 as well. PROV-165 changed only the
  assertion type (`u32` 0 -> `Option` None), not the lookup.

## Fix (chosen)

Remove the prefix branch from `find_model`. A catalog entry matches only by its key or by
its `id`, exactly (a duplicated id still matches nothing, as today). A near-miss is an
explicit miss: limits are unavailable, model validation reports "not found / Did you mean",
and the RPC display lookup falls back to the plain id (no borrowed capabilities).

## Alternatives rejected

- Keep prefix matching but require a separator (`gpt-4` vs `gpt-4-turbo`): still a guess;
  `claude-sonnet-4` would match `claude-sonnet-4-5` and borrow its limits.
- Keep prefix matching only for dated variants (`-YYYY-MM-DD`): the selector already
  carries catalog ids, so no caller needs it; it would remain a silent fallback.
- Change the test: the scenario states the user rule (a partial name does not authorize
  catalog capacity); the product is wrong, not the test.

## Wire / API changes

None. `ModelRegistry::get_model` keeps its signature; it simply stops answering prefixes.
