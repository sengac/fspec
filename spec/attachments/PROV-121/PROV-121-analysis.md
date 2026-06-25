# PROV-121 — OpenAI Profile Model Selection Ignores Profile baseUrl/apiKey

## Summary

Selecting an **openai-compatible PROFILE model** in the Rust TUI (e.g. `openai:qwen/qwen`,
a local vLLM/Ollama-style endpoint configured under
`~/.fspec/fspec-config.json` → `providers.openai.profiles.qwen`) fails when the agent
loop tries to dispatch a turn:

```
WARN codelet_agent_loop::agent_loop: [run_with_provider] Failed to get provider:
    [openai] Authentication error: OPENAI_API_KEY not set
ERROR codelet_agent_loop::agent_loop: Agent stream error for session ...:
    Failed to get provider: [openai] Authentication error: OPENAI_API_KEY not set
```

The profile's stored `baseUrl` (`http://192.168.0.50:8000`) and `apiKey` are **never
applied**, so the OpenAI client falls back to reading the `OPENAI_API_KEY` environment
variable — which is not set — and authentication fails.

## Observed in the logs (`~/.fspec/logs/fspec-combined.log.2026-06-24`)

The model was stored correctly:

```
INFO model_select: [MODEL-SELECT] handle_model_selected ENTER
    session_id=Some(...) provider_id=openai:qwen model_id=qwen
INFO RPC{...FspecService.set_session_model}: model_resolution:
    applying model via set_model_direct (profile/codex/custom) model="openai:qwen/qwen"
INFO model_select: [MODEL-SELECT] backend.set_session_model OK
```

…but dispatch then failed:

```
WARN codelet_agent_loop::agent_loop: [run_with_provider] Failed to get provider:
    [openai] Authentication error: OPENAI_API_KEY not set
```

The relevant config (keys redacted):

```json
"providers": {
  "openai": {
    "profiles": {
      "qwen": {
        "baseUrl": "http://192.168.0.50:8000",
        "apiKey": "test",
        "contextWindow": 200000
      }
    }
  }
}
```

## Root Cause (Rust)

### 1. `codelet/sessions/src/model_resolution.rs` — the profile env-bridge is missing

`apply_model_selection` correctly **detects** a profile model and parses provider/model:

```rust
// model_resolution.rs:39
let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
// "openai:qwen/qwen" -> registry_provider="openai", model_part="qwen"
```

But in the shared branch it only bridges credentials for **custom** providers:

```rust
// model_resolution.rs:65-101
if is_profile_model || is_codex_model || is_custom_model {
    pm.set_model_direct(registry_provider, model_part, None, None, None)?;  // profile STOPS here

    if is_custom_model {                                  // ← profiles EXCLUDED
        let facade = derive_facade_for_custom(registry_provider);
        pm.set_facade_override(facade.clone());
        apply_custom_provider_env_vars(registry_provider, model_part, facade.as_deref())?;
        //   ^ sets OPENAI_BASE_URL + OPENAI_API_KEY + OPENAI_MODEL
    }
    // ← NO `if is_profile_model { ... }` equivalent
}
```

Two defects:
- The profile **name** (`qwen`) is discarded: it calls `set_model_direct`, not
  `set_model_direct_with_profile`.
- The profile's `baseUrl`/`apiKey` are **never applied** to the env that the OpenAI
  client reads.

### 2. The model collapses to bare `ProviderType::OpenAI`

With no profile bridge, `current_provider` becomes plain `ProviderType::OpenAI`, whose
`as_str()` is `"openai"`.

### 3. Agent-loop dispatch routes to the plain cloud OpenAI arm

```rust
// codelet/agent-loop/src/agent_loop.rs:908
let result = match current_provider.as_str() {
    "claude" => ...,
    "openai" => {                                   // ← profile lands HERE
        match inner_session.provider_manager_mut().get_openai(session.id) { ... }
    }
    ...
    _ => { /* PROV-092 custom-provider arm — never reached for a profile */ }
};
```

### 4. `get_openai` reads only the `OPENAI_API_KEY` env var

```rust
// codelet/providers/src/manager.rs:815-831
pub fn get_openai(&self, session_id: uuid::Uuid) -> Result<OpenAIProvider, ProviderError> {
    ...
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| ProviderError::auth("openai", "OPENAI_API_KEY not set"))?;  // ← the error
    OpenAIProvider::from_api_key_with_session(&api_key, &model_id, session_id)
}
```

`OpenAIProvider` likewise reads the base URL only from `OPENAI_BASE_URL`
(`codelet/providers/src/openai.rs:169`). Neither ever consults
`providers.openai.profiles.qwen`.

The data IS available — `codelet/sessions/src/profile_sections.rs:196`
`load_local_server_profiles()` returns `LocalServerProfile { base_url, api_key, ... }` —
nothing pushes it into the env.

A stale comment at `manager.rs:586` even assumes *"TypeScript sets OPENAI_API_KEY and
OPENAI_BASE_URL ... AFTER the session was created"* — that is exactly the gap the
native Rust path fails to fill.

## TypeScript Reference (the correct behaviour to mirror)

TS injects the profile credentials into the process environment **before every** Rust
session/model operation, guarded by the presence of `profileConfig`.

### `src/tui/services/profileEnvironmentService.ts`

```ts
export function configureProfileEnvironment(config: ProfileConfig): void {
  process.env.OPENAI_BASE_URL = config.baseUrl;   // profile baseUrl
  process.env.OPENAI_API_KEY  = config.apiKey;    // profile apiKey (NOT the user's env var)
  if (config.contextWindow)   process.env.OPENAI_CONTEXT_WINDOW    = String(config.contextWindow);
  if (config.maxOutputTokens) process.env.OPENAI_MAX_OUTPUT_TOKENS = String(config.maxOutputTokens);
}
```

Called at the dispatch sites:
- `src/tui/services/modelSelectionService.ts:84` (on model switch)
- `src/tui/AgentView.tsx:3739` and `:3957` (before `createSession`)

```ts
if (selection.profileConfig) {
  configureProfileEnvironment(selection.profileConfig);
}
```

### Parsing — `src/tui/utils/model-selection.ts` `parseModelString()`

- Profile format `provider:profile/modelId` → `profileName != null` ⇒ **PROFILE**
- Cloud format `provider/modelId` → `profileName == null` ⇒ **CLOUD**

### Config read — `src/utils/profile-management.ts` / `src/utils/provider-config.ts`

`loadProviderProfiles('openai')` → `config.providers.openai.profiles` →
`Record<name, { baseUrl, apiKey, contextWindow?, maxOutputTokens?, ... }>`.

### The discriminator

| | Cloud OpenAI | Profile openai-compatible |
|---|---|---|
| Model string | `openai/gpt-4` | `openai:qwen/qwen` |
| `profileName` | `null` | `"qwen"` |
| `profileConfig` | absent | present `{baseUrl, apiKey, ...}` |
| Credentials | Codex/OAuth (`PROVIDER_ENV_VARS.openai = []`) | `OPENAI_BASE_URL`/`OPENAI_API_KEY` overwritten from profile |

TS deliberately keeps cloud OpenAI's env-var list **empty** so cloud env resolution
never fires for the profile path.

## Proposed Fix (Rust)

In `codelet/sessions/src/model_resolution.rs`, add an `is_profile_model` branch that
mirrors TS `configureProfileEnvironment` and the existing `is_custom_model` bridge:

1. Load the matching profile via `load_local_server_profiles()` for
   `(provider=registry_provider, profile=<the name between ':' and '/'>)`.
   - Extract the profile name (currently discarded) — it is the segment between `:`
     and the first `/`.
2. Apply the profile's credentials to the env the OpenAI client reads:
   - `OPENAI_BASE_URL` ← `profile.base_url`
   - `OPENAI_API_KEY` ← `profile.api_key`
   - optionally `OPENAI_CONTEXT_WINDOW` / `OPENAI_MAX_OUTPUT_TOKENS` from the profile.
   - Prefer a small dedicated helper (e.g. `apply_profile_env_vars`) for testability,
     analogous to `apply_custom_provider_env_vars`.
3. Call `set_model_direct_with_profile(...)` (not `set_model_direct`) so the profile
   name is preserved on the manager (`selected_profile_name`), keeping the composite
   `openai:qwen/qwen` round-trip intact.
4. Apply the same bridge on BOTH paths that resolve a model:
   - mid-session `set_session_model` → `apply_model_selection`
   - `SessionManager::create_session_with_id` (the create path mirrors the same
     profile detection — ensure parity there too).

### Acceptance-relevant behaviour to preserve
- Cloud OpenAI selections (`openai/<model>`, no profile) must NOT be affected.
- Custom registered providers (`is_custom_model`) keep their existing bridge.
- Codex selections unaffected.
- No hardcoded anthropic/claude fallback (PROV-101 invariant).

## Files of Record

### Rust (port — to fix)
- `codelet/sessions/src/model_resolution.rs` — selection + (missing) profile env bridge
- `codelet/sessions/src/profile_sections.rs:196` — `load_local_server_profiles()` /
  `LocalServerProfile { base_url, api_key, ... }`
- `codelet/providers/src/manager.rs:815-831` — `get_openai` (reads only env)
- `codelet/providers/src/manager.rs:573-606` — `set_model_direct_with_profile`,
  `selected_profile_name`
- `codelet/providers/src/openai.rs:162-169` — base-url-from-env constructor
- `codelet/agent-loop/src/agent_loop.rs:908` — dispatch match on provider name
- `codelet/providers/src/custom/management.rs` — `apply_custom_provider_env_vars`
  (the pattern to mirror)

### TypeScript (reference — correct behaviour)
- `src/tui/services/profileEnvironmentService.ts` — `configureProfileEnvironment`
- `src/tui/utils/model-selection.ts` — `parseModelString` / `buildModelString`
- `src/utils/profile-management.ts`, `src/utils/provider-config.ts` — `ProfileConfig`
- `src/tui/services/modelSelectionService.ts`, `src/tui/services/sessionService.ts` —
  cloud vs profile dispatch branching
- `src/utils/provider-registry.ts`, `src/utils/credentials.ts` — `openai` has
  `envVar:''`, `requiresApiKey:false`, `PROVIDER_ENV_VARS.openai = []`

## Reproduction

1. Add an openai profile to `~/.fspec/fspec-config.json`:
   `providers.openai.profiles.qwen = { baseUrl, apiKey, contextWindow }`.
2. Ensure `OPENAI_API_KEY` is **unset** in the environment.
3. Launch the TUI, open `/model`, select the `openai:qwen` → `qwen` row.
4. Send a message → agent loop fails with
   `[openai] Authentication error: OPENAI_API_KEY not set`.

Expected: the turn dispatches using the profile's `baseUrl` + `apiKey`.
