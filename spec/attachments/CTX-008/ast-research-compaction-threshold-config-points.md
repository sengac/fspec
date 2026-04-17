# CTX-008 AST Research: Compaction Threshold TUI Configuration Points

## 1. TypeScript Types to Modify

### ProfileConfig (src/utils/provider-config.ts:60)
```typescript
export interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
  customModels?: CustomModelDefinition[];
}
```
**Action:** Add `compactionThreshold?: CompactionThresholdConfig`

### CustomModelDefinition (src/utils/provider-config.ts:80)
```typescript
export interface CustomModelDefinition {
  id: string;
  displayName?: string;
  facade?: 'openai' | 'codex' | 'claude' | 'gemini' | 'zai';
  contextWindow?: number;
  maxOutputTokens?: number;
  reasoning?: boolean;
  hasVision?: boolean;
}
```
**Action:** Add `compactionThreshold?: CompactionThresholdConfig`

### ModelSelection (src/tui/types/provider.ts:50)
```typescript
export interface ModelSelection {
  providerId: string;
  modelId: string;
  apiModelId: string;
  displayName: string;
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number;
  maxOutput: number;
  profileName?: string;
  profileConfig?: ProfileConfig;
  facade?: string;
}
```
**Action:** Add `compactionThreshold?: CompactionThresholdConfig`

## 2. NAPI Functions (Rust side)

### session_set_model (codelet/napi/src/session_manager.rs:6659)
```rust
pub async fn session_set_model(
  session_id: String, provider_id: String, model_id: String,
  context_window: Option<u32>, max_output_tokens: Option<u32>
) -> Result<()>
```
**Action:** Add `compaction_threshold_type: Option<String>`, `compaction_threshold_value: Option<u32>`

### session_set_model_profile (codelet/napi/src/session_manager.rs:6735)
```rust
pub async fn session_set_model_profile(
  session_id: String, provider_id: String, model_id: String,
  context_window: Option<u32>, max_output_tokens: Option<u32>,
  facade_override: Option<String>
) -> Result<()>
```
**Action:** Add `compaction_threshold_type: Option<String>`, `compaction_threshold_value: Option<u32>`

## 3. NAPI Call Sites (TypeScript side)

### modelSelectionService.ts
- Line 98: `await sessionSetModelProfile(sessionId, ..., selection.facade ?? null)` — add compaction threshold params
- Line 109: `await sessionSetModelProfile(sessionId, ..., null)` — add compaction threshold params
- Line 120: `await sessionSetModel(sessionId, ..., selection.maxOutput)` — add compaction threshold params

## 4. Form Field Constants

### providerSettings.ts
```typescript
export const PROFILE_FORM_FIELDS: Array<keyof ProfileConfig> = [
  'baseUrl', 'apiKey', 'contextWindow', 'maxOutputTokens',
];
```
**Action:** Add `'compactionThreshold'`

### customModelForm.ts
```typescript
export const CUSTOM_MODEL_FORM_FIELDS: CustomModelFormField[]
```
**Action:** Add compactionThreshold field between maxOutputTokens and reasoning

## 5. Input Handler

### profileFormModeHandler.ts:149
```typescript
if (field === 'contextWindow' || field === 'maxOutputTokens') {
  const num = parseInt(newValue, 10);
  return { ...prev, [field]: isNaN(num) ? undefined : num };
}
```
**Action:** Add handling for compactionThreshold field (parse % or plain number)

## 6. Existing Rust Infrastructure (CTX-007)

### ProviderManager (codelet/providers/src/manager.rs)
- `set_compaction_threshold_override(&mut self, config: Option<(String, u64)>)` — already exists
- `compaction_threshold_override(&self) -> Option<(&str, u64)>` — already exists

### resolve_compaction_threshold (codelet/cli/src/compaction_threshold.rs)
- Already resolves with user_config override priority — already exists

## 7. Custom Model Save (useCustomModelFormState.ts:109)

```typescript
const definition: CustomModelDefinition = {
  id: values.id.trim(),
  ...(values.displayName?.trim() && { displayName: values.displayName.trim() }),
  ...(values.facade && { facade: values.facade }),
  ...(values.contextWindow && { contextWindow: values.contextWindow }),
  ...(values.maxOutputTokens && { maxOutputTokens: values.maxOutputTokens }),
  ...(values.reasoning !== undefined && { reasoning: values.reasoning }),
  ...(values.hasVision !== undefined && { hasVision: values.hasVision }),
};
```
**Action:** Add compactionThreshold to the saved definition
