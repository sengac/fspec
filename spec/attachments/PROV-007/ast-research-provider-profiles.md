# AST Research: Provider Profiles Implementation

## Work Unit: PROV-007 - Provider Configuration Persistence and TUI Display

### Research Date: 2026-02-23

---

## 1. Provider Configuration Module (`src/utils/provider-config.ts`)

### Current Interface Structure:

```typescript
interface ProviderConfig {
  enabled?: boolean;
  baseUrl?: string;
  defaultModel?: string;
  authMethod?: AuthMethod;
  // Azure-specific
  endpoint?: string;
  apiVersion?: string;
  // Additional headers
  headers?: Record<string, string>;
}
```

### Required Changes for Profiles:

```typescript
// New interface to add:
interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
}

// Extend ProviderConfig:
interface ProviderConfig {
  // ... existing fields
  profiles?: Record<string, ProfileConfig>;
}
```

### Existing Functions to Extend:

- `loadProviderConfig(providerId: string)` - Load provider config from fspec-config.json
- `saveProviderConfig(providerId, config)` - Save provider config to fspec-config.json

### New Functions Needed:

- `loadProviderProfiles(providerId: string): Promise<Record<string, ProfileConfig>>`
- `saveProfile(providerId: string, profileName: string, profile: ProfileConfig): Promise<void>`
- `deleteProfile(providerId: string, profileName: string): Promise<void>`
- `getProfile(providerId: string, profileName: string): Promise<ProfileConfig | undefined>`

---

## 2. Model Selector (`src/tui/components/AgentView.tsx`)

### Current Structure:

```typescript
interface ProviderSection {
  providerId: string;      // "anthropic"
  providerName: string;    // "Anthropic"
  internalName: string;    // "claude"
  models: NapiModelInfo[]; // From models.dev
  hasCredentials: boolean;
}
```

### Key State Variables:

- `providerSections: ProviderSection[]` - List of provider sections
- `expandedProviders: Set<string>` - Which sections are expanded
- `selectedSectionIdx: number` - Currently selected section
- `selectedModelIdx: number` - Currently selected model (-1 for section header)

### Model Selection Flow:

1. `/model` command → `setShowModelSelector(true)`
2. Section data built from `modelsListAll()` → filtered to `hasCredentials && toolCall`
3. User selects model → `handleModelSelection(section, model)` called
4. Session created with `modelPath = provider/modelId`

### Changes Needed for Profiles:

1. Add profile sections alongside cloud provider sections
   - Profile sections named like "openai: work-vllm"
   - Profile sections fetch models via `modelsListLocalOpenai(baseUrl)`
   
2. Update `ProviderSection` interface:
```typescript
interface ProviderSection {
  providerId: string;
  providerName: string;
  internalName: string;
  models: NapiModelInfo[];
  hasCredentials: boolean;
  // New fields for profiles:
  isProfile?: boolean;
  profileName?: string;
  profileConfig?: ProfileConfig;
}
```

3. When selecting from profile section:
   - Set environment variables from profile config
   - Pass to session creation

---

## 3. Provider Settings View (`/provider` command)

### Current Implementation:

- Shows list of providers with API key status
- Allows editing/deleting API keys
- Uses `providerStatuses` state

### Current Flow:

1. `/provider` command → `setShowSettingsTab(true)`
2. Displays `filteredSettingsProviders` (list of provider IDs)
3. Each provider shows:
   - Name (from registry)
   - Key status (configured/not configured)
   - Edit/Test/Delete actions

### Changes Needed for Profile CRUD:

1. Restructure view to show profiles per provider
2. Add profile management actions:
   - Create profile
   - Edit profile settings (baseUrl, apiKey, contextWindow, maxOutputTokens)
   - Delete profile
3. Show profile list for each provider

---

## 4. Session Service (`src/tui/services/sessionService.ts`)

### Current Session Creation:

```typescript
export async function createSession(options: CreateSessionOptions): Promise<CreateSessionResult> {
  const { modelPath, project, name } = options;
  // ...
  await sessionManagerCreateWithId(persistedSession.id, modelPath, project, sessionName);
}
```

### Session Creation from NAPI (`sessionManagerCreateWithId`):

The Rust side reads environment variables for provider configuration:
- `OPENAI_BASE_URL` - Base URL for OpenAI-compatible servers
- `OPENAI_API_KEY` - API key
- `OPENAI_CONTEXT_WINDOW` - Context window size
- `OPENAI_MAX_OUTPUT_TOKENS` - Max output tokens

### Changes Needed:

When creating session from profile, set environment variables BEFORE calling `sessionManagerCreateWithId`:

```typescript
// Proposed approach:
export async function createSessionWithProfile(
  options: CreateSessionOptions & { 
    profileConfig?: ProfileConfig 
  }
): Promise<CreateSessionResult> {
  if (options.profileConfig) {
    process.env.OPENAI_BASE_URL = options.profileConfig.baseUrl;
    process.env.OPENAI_API_KEY = options.profileConfig.apiKey;
    if (options.profileConfig.contextWindow) {
      process.env.OPENAI_CONTEXT_WINDOW = String(options.profileConfig.contextWindow);
    }
    if (options.profileConfig.maxOutputTokens) {
      process.env.OPENAI_MAX_OUTPUT_TOKENS = String(options.profileConfig.maxOutputTokens);
    }
  }
  // Then create session as normal
}
```

---

## 5. NAPI Bindings

### Existing Binding for Local Model Listing:

```typescript
// From codelet/napi/index.d.ts:
export declare function modelsListLocalOpenai(baseUrl: string): Promise<Array<string>>;
```

This function is already implemented and returns model IDs from a local server's `/v1/models` endpoint.

### Usage in TUI:

When building profile sections, call `modelsListLocalOpenai(profile.baseUrl)` to get available models.

---

## 6. Config File Structure

### Target Structure (`~/.fspec/fspec-config.json`):

```json
{
  "providers": {
    "openai": {
      "profiles": {
        "work-vllm": {
          "baseUrl": "http://work:8888",
          "apiKey": "local-key",
          "contextWindow": 32768,
          "maxOutputTokens": 8192
        },
        "home-ollama": {
          "baseUrl": "http://localhost:11434",
          "apiKey": "local-key"
        }
      }
    }
  }
}
```

---

## 7. Implementation Order

### Phase 1: Profile Config Layer
1. Add `ProfileConfig` interface to `provider-config.ts`
2. Add profile CRUD functions
3. Write tests for profile persistence

### Phase 2: Model Selector Profiles
1. Extend `ProviderSection` interface
2. Build profile sections from config
3. Fetch models from profile baseUrl using `modelsListLocalOpenai`
4. Handle unreachable servers gracefully

### Phase 3: Session Creation with Profiles
1. Pass profile config through model selection
2. Set env vars before session creation
3. Test end-to-end flow

### Phase 4: Provider Screen Profile CRUD
1. Restructure `/provider` view
2. Add profile create/edit/delete UI
3. Wire up to profile config functions

---

## 8. Error Handling Considerations

1. **Unreachable local server**: Show "(unreachable)" in model selector, don't block UI
2. **Invalid profile config**: Validate required fields (baseUrl, apiKey) on save
3. **Model fetch failure**: Graceful degradation, show error in section
4. **Session creation failure**: Clear error message indicating profile issue

---

## 9. Key Files to Modify

| File | Changes |
|------|---------|
| `src/utils/provider-config.ts` | Add ProfileConfig, CRUD functions |
| `src/tui/components/AgentView.tsx` | Profile sections in model selector |
| `src/tui/services/sessionService.ts` | Set env vars from profile |
| New test files | Unit tests for all new functionality |

---

## 10. Dependencies

- PROV-006: `modelsListLocalOpenai()` NAPI binding (DONE)
- Existing config infrastructure in `src/utils/config.ts`
- Existing provider registry in `src/utils/provider-config.ts`
