# PROV-078 — End-to-end smoke tests for the custom-provider pipeline

## Problem

PROV-062 through PROV-077 each cover their own slice but no test exercises the **full** user journey from zero to "chatting with a custom provider". Past regressions (PROV-067 NAPI exposure, PROV-069 dispatch) weren't caught by surface-level unit tests. A single smoke test that runs the whole stack would catch:

- `fspec_home()` semantics drift (PROV-071)
- NAPI shape mismatches (PROV-072)
- Registry cache invalidation gaps (PROV-073)
- Settings Screen rendering gaps (PROV-074)
- Model Selector omission (PROV-075)
- Slash-command wiring (PROV-076)
- File watcher stalls (PROV-077)

## Test scenarios (each is one `.test.ts` in `src/tui/__tests__/`)

### 1. Zero → provider registered via `/provider init` → selectable in `/model`

```
given FSPEC_HOME = tmpdir
and providers dir is empty
when I run `/provider init my-test --template=openai-compatible --scope=global`
and I fill the wizard with baseUrl=http://127.0.0.1:1234/v1, apiKeyEnvVar=MY_TEST_API_KEY, model=fast (id=fast-v1, ctx=8000, maxOut=1024)
then a file <FSPEC_HOME>/providers/my-test.json exists with the expected fields
and the settings screen shows `my-test` with facade=openai, origin=global
and `modelsListAll()` includes my-test / fast
and selecting my-test/fast in /model triggers `session_set_model_profile` which calls `apply_custom_provider_env_vars`
```

### 2. External file drop → watcher triggers registry refresh

```
given the settings screen is open
when I write a new JSON file directly to <FSPEC_HOME>/providers/external.json
then within 500 ms the settings screen shows `external` without user input
and a toast appears if the screen is not focused
```

### 3. Project-local override shadows global

```
given a global provider at <FSPEC_HOME>/providers/shared.json with baseUrl=A
and a project-local provider at <cwd>/.fspec/providers/shared.json with baseUrl=B
when I open /provider and expand `shared`
then baseUrl=B is shown
and origin=project is displayed
and `deleteProvider('shared', 'project')` removes only the project file
and after deletion, origin=global with baseUrl=A is displayed
```

### 4. Invalid JSON → graceful degradation

```
given a malformed <FSPEC_HOME>/providers/broken.json
when I open /provider
then the row shows `⚠ invalid` with the parse error
and only the `[Open file]` and `[Delete]` actions are enabled
and the provider is not in `/model` sections
and selecting it is impossible (no keyboard shortcut reaches selectable state)
```

### 5. Rhai-full template creates valid skeleton

```
when I run `/provider init rhai-demo --template=rhai-full --scope=global`
and I fill the wizard
then <FSPEC_HOME>/providers/rhai-demo.json and <FSPEC_HOME>/providers/rhai-demo.rhai exist
and `validateProvider('rhai-demo')` succeeds (all 7 lifecycle functions defined)
and `testProvider('rhai-demo')` fails with a domain-specific error (because stubs `throw`) — NOT a compile error
```

### 6. Delete round-trips through registry cache

```
given a custom provider `del-me`
when I run `/provider delete del-me`
and I confirm
then the JSON file is removed
and the registry cache is invalidated
and `getProviderRegistry()` no longer contains `del-me`
and `/model` sections no longer contain `del-me`
```

### 7. `/provider paths` prints correct paths

```
given FSPEC_HOME unset and HOME=/home/alice
when I run `/provider paths`
then output includes:
  base:        /home/alice/.fspec
  credentials: /home/alice/.fspec/credentials
  providers:   /home/alice/.fspec/providers
  project:     <cwd>/.fspec/providers
```

## Rust-side smoke test

One Rust integration test in `codelet/providers/tests/custom_provider_end_to_end_test.rs`:

```
given an FspecHomeGuard tmpdir
when I scaffold a provider via init_provider(...)
and call discover_provider_configs()
and invoke apply_custom_provider_env_vars(name, model_id, None)
then OPENAI_BASE_URL / OPENAI_API_KEY / OPENAI_MODEL are set
and ProviderManager::with_provider(name) succeeds
and set_model_direct(name, model_id, ...) succeeds
```

This ensures the Rust side of the pipeline is healthy even when the TS layer is absent.

## Helpers needed

- `FSPEC_HOME` tmpdir fixture for TS tests — extend `src/test-helpers/home-directory-fixture.ts`.
- `renderProviderSettingsScreen()` helper (exists) + a `waitForWatcherEvent` helper.
- `drainSlashCommand(input: string)` helper that simulates running a slash command through the TUI reducer.

## Acceptance summary

- 7 TS integration tests pass, covering the full `/provider` + `/model` + watcher loop.
- 1 Rust end-to-end test passes, covering scaffold → discover → apply env vars → set model.
- Tests run under 10 s total (watcher debounce fixtures use short windows).
- Failures produce actionable diffs with named identifiers, not opaque timeouts.

## Dependencies

- PROV-071, PROV-072, PROV-073, PROV-074, PROV-075, PROV-076, PROV-077 — all prior cards in the epic must be completed before this card can reach `done`.

## References

- `src/test-helpers/home-directory-fixture.ts` (extend)
- `codelet/providers/tests/custom_provider_manager_integration_test.rs` (existing pattern)
- `spec/features/custom-provider-manager-integration.feature` (PROV-067 acceptance criteria)
