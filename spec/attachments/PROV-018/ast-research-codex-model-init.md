# AST Research: Codex Model Initialization

## Analyzed Files

### src/tui/services/modelInitializationService.ts
- `buildCloudSections()` - Main function that filters cloud models by credentials
  - Iterates `NapiProviderModels[]` from `modelsListAll()`
  - Maps each provider to registry via `mapModelsDevToRegistryId()`
  - Checks `hasCredentials = registryEntry?.requiresApiKey === false || !!providerConfig.apiKey`
  - Filters `pm.models.filter(m => m.toolCall)` for tool-call-capable models
  - Returns only sections where `hasCredentials === true`
  - **BUG**: OpenAI section (containing codex models) filtered out when no OPENAI_API_KEY

### src/tui/utils/provider-mapping.ts
- `mapProviderIdToInternal()` - 'codex' maps to 'codex' (default identity)
- `mapModelsDevToRegistryId()` - 'codex' maps to 'codex' (default identity)
- No changes needed - identity mapping already works

### src/utils/provider-config.ts
- `SUPPORTED_PROVIDERS` includes 'codex'
- Codex registry entry: `{ id: 'codex', requiresApiKey: false }`
- Already configured correctly

### src/tui/hooks/useProviderSettingsState.ts
- Already checks `codexOauthGetTokens()` for codex provider
- Provider settings panel shows OAuth status correctly

## Key Finding
The `buildCloudSections()` function needs to:
1. Detect Codex OAuth tokens via `codexOauthGetTokens()`
2. Extract codex models (ID contains 'codex') from OpenAI provider
3. Create synthetic "Codex (ChatGPT)" section with `providerId: 'codex'`
4. The codex registry entry has `requiresApiKey: false`, so `hasCredentials` will be true
