# PROV-018 Investigation: Codex Models Not Showing in Model Selector

## Screenshots

- `/provider` screen: Shows `Codex (ChatGPT) ✓ •••••••• [OAuth]` — OAuth connection is active and valid
- `/model` screen: Shows only Anthropic (19 models), Google (21 models), Z.AI (9 models) — **zero Codex models**

## Root Cause Analysis

### The Data Is There

The local models.dev cache (`~/.fspec/cache/models.json`) has 8 Codex models under the **"openai"** provider:

| Model ID | Name | Context | Output |
|---|---|---|---|
| gpt-5.3-codex | GPT-5.3 Codex | 400K | 128K |
| gpt-5.2-codex | GPT-5.2 Codex | 400K | 128K |
| gpt-5.1-codex | GPT-5.1 Codex | 400K | 128K |
| gpt-5.1-codex-max | GPT-5.1 Codex Max | 400K | 128K |
| gpt-5.1-codex-mini | GPT-5.1 Codex mini | 400K | 128K |
| gpt-5-codex | GPT-5-Codex | 400K | 128K |
| codex-mini-latest | Codex Mini | 200K | 100K |
| gpt-5.3-codex-spark | GPT-5.3 Codex Spark | 128K | 32K |

All have `tool_call: true` so they pass the tool-call filter.

### The Filtering Bug

In `src/tui/services/modelInitializationService.ts`, `buildCloudSections()`:

```typescript
const hasCredentials =
  registryEntry?.requiresApiKey === false || !!providerConfig.apiKey;
// ...
return sectionsWithCreds.filter(s => s.hasCredentials);
```

For the **"openai"** provider from models.dev:
1. `registryId = mapModelsDevToRegistryId('openai')` → `'openai'`
2. `registryEntry = getProviderRegistryEntry('openai')` → `{ requiresApiKey: true, envVar: 'OPENAI_API_KEY' }`
3. `providerConfig = await getProviderConfig('openai')` → no OPENAI_API_KEY set → `apiKey = undefined`
4. `hasCredentials = false || false` → **`false`**
5. **The entire OpenAI section (including all 8 codex models) is filtered out**

Meanwhile, our separate `codex` entry in `SUPPORTED_PROVIDERS` has `requiresApiKey: false`, but models.dev doesn't have a separate "codex" provider — the models live under "openai". So `models_list_all()` never returns a "codex" provider section.

**Result**: Codex models exist in the data but are discarded because they're nested under the OpenAI provider, and the user doesn't have an OpenAI API key (they have OAuth tokens instead).

### The Provider Screen Works Fine

`useProviderSettingsState.ts` correctly detects OAuth tokens for the codex provider:

```typescript
if (providerId === 'codex') {
  const tokens = codexOauthGetTokens();
  if (tokens) {
    hasOAuthTokens = true;
    status = { hasKey: true, maskedKey: '••••••••', source: 'OAuth' };
  }
}
```

This is why `/provider` shows `Codex (ChatGPT) ✓ •••••••• [OAuth]`. But the provider screen and model screen are disconnected — the model screen has no codex-aware logic.

## How OpenCode Solves This

In `/tmp/opencode/packages/opencode/src/plugin/codex.ts`:

1. **Codex is NOT a separate provider** — it's a **plugin** that hooks into the `openai` provider
2. When OAuth is the auth method, the plugin's `loader`:
   - **Filters** the OpenAI model list to only codex models (whitelist: `gpt-5.1-codex-max`, `gpt-5.2-codex`, `gpt-5.3-codex`, etc.)
   - **Injects** any missing models (e.g., `gpt-5.3-codex`)
   - **Zeroes out costs** (included with subscription)
   - Returns a **custom `fetch` function** that handles token refresh + URL rewriting

3. The codex models appear under the OpenAI provider in the model list, but only when OAuth tokens exist
4. The model selection and provider are the same — it's just the auth method and available model subset that changes

## What Needs to Change

### `modelInitializationService.ts` → `buildCloudSections()`

When Codex OAuth tokens exist:
1. Extract codex-specific models from the OpenAI provider's model list (models where ID contains "codex")
2. Create a **separate "Codex (ChatGPT)" section** with `hasCredentials: true`
3. This section appears in the model selector, allowing selection of `gpt-5.3-codex`, etc.

### Session Creation Path

When a Codex model is selected:
- The `providerId` sent to the Rust session manager needs to be `"codex"` (not `"openai"`)
- The `CodexProvider` (Rust) already handles OAuth tokens, URL rewriting to `chatgpt.com/backend-api/codex/responses`, and token refresh via `RefreshingCodexClient`
- The `CODEX_MODEL` env var needs to be set to the selected model ID

### Provider Mapping

`src/tui/utils/provider-mapping.ts` may need updates to correctly map the synthetic "codex" section back to the right provider for session creation.

## Files Involved

| File | Role |
|---|---|
| `src/tui/services/modelInitializationService.ts` | Main fix: extract codex models from openai section when OAuth tokens exist |
| `src/tui/hooks/useModelSelectorState.ts` | May need updates for codex section handling |
| `src/tui/utils/provider-mapping.ts` | Provider ID mapping for codex |
| `src/utils/provider-config.ts` | Codex registry entry (already exists, `requiresApiKey: false`) |
| `codelet/napi/src/codex_oauth.rs` | NAPI binding for `codexOauthGetTokens()` |
| `codelet/providers/src/codex/mod.rs` | Rust CodexProvider (already handles OAuth) |

## Dependencies

- Depends on: PROV-013, PROV-014, PROV-015, PROV-016, PROV-017 (all done)
- Blocks: Actual use of Codex subscription through fspec
