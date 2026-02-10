# AST Research: TypeScript Credential Usage for CONFIG-005

## Purpose
Analyze how TypeScript currently handles credentials to understand what needs to change.

## Research Date
2026-02-10

## Current TypeScript Credential Flow

### 1. Credential Resolution in TypeScript

**File:** `src/utils/credentials.ts`

```typescript
// Priority chain (currently in TypeScript):
export async function getProviderConfig(providerId: string): Promise<ProviderConfigResult> {
  // 1. Try credentials file first
  const credentials = await loadCredentials();
  const providerCred = credentials.providers[providerId];
  if (providerCred?.apiKey) {
    return { apiKey: providerCred.apiKey, source: 'file' };
  }

  // 2. Try environment variables
  const envVars = PROVIDER_ENV_VARS[providerId];
  if (envVars) {
    for (const envVar of envVars) {
      if (process.env[envVar]) {
        return { apiKey: process.env[envVar], source: 'env' };
      }
    }
  }

  // 3. Try .env file
  // ...
}
```

### 2. Session Creation in AgentView

**File:** `src/tui/components/AgentView.tsx:2760-2780`

```typescript
try {
  // CONFIG-004: Get API key from credentials before creating Rust session
  const providerId = modelPath.split('/')[0] || '';
  let apiKey: string | undefined;
  if (providerId) {
    const providerConfig = await getProviderConfig(providerId);
    apiKey = providerConfig.apiKey;
    if (apiKey) {
      logger.debug(
        `[AgentView] Using API key from ${providerConfig.source} for provider ${providerId}`
      );
    }
  }

  await sessionManagerCreateWithId(
    activeSessionId,
    modelPath,
    project,
    sessionName,
    apiKey  // <-- THIS IS THE PROBLEM - passed once, never refreshed
  );
}
```

### 3. Provider Environment Variable Mapping

**File:** `src/utils/credentials.ts:49-70`

```typescript
const PROVIDER_ENV_VARS: Record<string, string[]> = {
  anthropic: ['ANTHROPIC_API_KEY', 'CLAUDE_CODE_OAUTH_TOKEN'],
  openai: ['OPENAI_API_KEY'],
  cohere: ['COHERE_API_KEY'],
  gemini: ['GOOGLE_GENERATIVE_AI_API_KEY', 'GEMINI_API_KEY'],
  mistral: ['MISTRAL_API_KEY'],
  xai: ['XAI_API_KEY'],
  together: ['TOGETHER_API_KEY'],
  huggingface: ['HUGGINGFACE_API_KEY', 'HF_TOKEN'],
  openrouter: ['OPENROUTER_API_KEY'],
  groq: ['GROQ_API_KEY'],
  ollama: ['OLLAMA_API_KEY'],
  deepseek: ['DEEPSEEK_API_KEY'],
  perplexity: ['PERPLEXITY_API_KEY'],
  moonshot: ['MOONSHOT_API_KEY'],
  hyperbolic: ['HYPERBOLIC_API_KEY'],
  mira: ['MIRA_API_KEY'],
  galadriel: ['GALADRIEL_API_KEY'],
  azure: ['AZURE_OPENAI_API_KEY'],
  voyageai: ['VOYAGEAI_API_KEY'],
  zai: ['ZAI_API_KEY', 'ZAI_PLAN_API_KEY'],
};
```

## Changes Required

### TypeScript Side

1. **Remove credential passing to NAPI:**
   - `AgentView.tsx`: Remove `apiKey` parameter from `sessionManagerCreateWithId()` call
   - `sessionService.ts`: Update any other session creation calls

2. **Keep save/delete operations:**
   - `saveCredential()` - stays in TypeScript (writes to file)
   - `deleteCredential()` - stays in TypeScript (removes from file)

3. **Add NAPI reload call after save:**
   ```typescript
   export async function saveCredential(providerId: string, apiKey: string): Promise<void> {
     // ... existing save logic ...
     
     // NEW: Notify Rust to reload credentials
     await credentialsReload();
   }
   ```

4. **Remove/simplify getProviderConfig:**
   - No longer needed for NAPI calls
   - May keep for TUI display purposes (showing masked key)

### NAPI Binding Changes

1. **Remove api_key parameter:**
   ```typescript
   // Before
   await sessionManagerCreateWithId(id, model, project, name, apiKey);
   
   // After
   await sessionManagerCreateWithId(id, model, project, name);
   ```

2. **Add new NAPI functions:**
   ```typescript
   export function credentialsReload(): Promise<boolean>;
   // No credentialsResolve() exposed - credentials stay in Rust
   ```

## Files to Modify

| File | Change |
|------|--------|
| `src/tui/components/AgentView.tsx` | Remove apiKey parameter from sessionManagerCreateWithId call |
| `src/utils/credentials.ts` | Add credentialsReload() call after saveCredential() |
| `src/bindings/codelet-napi.d.ts` | Update sessionManagerCreateWithId signature, add credentialsReload |
