# PROV-029 Post-Review Fixes

**Date:** 2026-03-03  
**Reviewer:** Claude Code (fspec review)  
**Status:** PROV-029 marked done but has unresolved issues

---

## 🔴 FIX 1: Type System Integrity — PanelMode vs HookMode Split (CRITICAL)

### Problem

The hook (`useProviderSettingsState`) stores modes like `delete-profile`, `create-profile`, `edit-profile` in its internal `useState<PanelMode>`. But `PanelMode` (defined in `ProviderSettingsPanel.tsx`) does NOT include those variants — it has `delete-confirm`, `profile-form`, etc. instead.

The mapper (`providerSettingsModeMapper.ts`) translates between the two. But TypeScript **never catches** when the hook calls `setMode({ type: 'delete-profile', ... })` even though `'delete-profile'` is not a valid `PanelMode` type. This is because the `bridge/**/*` include in `tsconfig.json` causes TS6059 errors that make `tsc --noEmit` exit code 2, masking all type errors in source files.

Similarly, `deleteConfirmModeHandler.ts` accepts `mode: PanelMode` but checks `mode.type === 'delete-profile'` — which is not a `PanelMode` variant.

### Where the correct types already exist

`src/test-helpers/provider-settings-state-fixtures.ts` lines 26-31 already defines:

```typescript
export type SettingsViewMode =
  | { type: 'list' }
  | { type: 'edit-api-key'; providerId: string }
  | { type: 'create-profile'; providerId: string }
  | { type: 'edit-profile'; providerId: string; profileName: string }
  | { type: 'delete-profile'; providerId: string; profileName: string };
```

This is the HOOK-side type that is missing from production code.

### Fix

1. **Create `src/tui/types/settingsMode.ts`** — Define `HookMode` (or `SettingsViewMode`) as the complete union of all modes the hook can be in. Include the OAuth modes too:
   ```typescript
   export type HookMode =
     | { type: 'list' }
     | { type: 'edit-api-key'; providerId: string }
     | { type: 'delete-api-key'; providerId: string }
     | { type: 'disconnect-oauth'; providerId: string }
     | { type: 'create-profile'; providerId: string }
     | { type: 'edit-profile'; providerId: string; profileName: string }
     | { type: 'delete-profile'; providerId: string; profileName: string }
     | { type: 'oauth-browser-waiting'; providerId: string }
     | { type: 'oauth-device-waiting'; providerId: string; userCode: string; verificationUrl: string }
     | { type: 'oauth-success'; providerId: string }
     | { type: 'oauth-error'; providerId: string; error: string }
     | { type: 'oauth-headless-code-entry'; providerId: string; authorizeUrl: string; pkceVerifier: string; codeInput: string };
   ```

2. **Change `useProviderSettingsState.ts`:**
   - `useState<PanelMode>` → `useState<HookMode>`
   - `mode: PanelMode` in interface → `mode: HookMode`
   - `setMode: (mode: PanelMode) => void` → `setMode: (mode: HookMode) => void`

3. **Change `deleteConfirmModeHandler.ts`:**
   - `mode: PanelMode` → `mode: HookMode`
   - Now `mode.type === 'delete-profile'` is type-safe

4. **Change `providerSettingsModeMapper.ts`:**
   - Input: `HookMode` → Output: `PanelMode`
   - This is the ONLY place that maps between the two types

5. **Change `useProviderSettingsInput.ts`:**
   - `mode` from hook is now `HookMode`, passed to handlers as `HookMode`

6. **Keep `PanelMode` in `ProviderSettingsPanel.tsx` unchanged** — it's the rendering type

7. **Update `provider-settings-state-fixtures.ts`** — import `HookMode` from production code instead of redefining it

8. **Update test file** — remove `as PanelMode` casts that hide the mismatch, use proper `HookMode` type

### Files to change

| File | Change |
|------|--------|
| `src/tui/types/settingsMode.ts` | NEW — `HookMode` type |
| `src/tui/hooks/useProviderSettingsState.ts` | `PanelMode` → `HookMode` for internal state + interface |
| `src/tui/inputHandlers/deleteConfirmModeHandler.ts` | `PanelMode` → `HookMode` for mode param |
| `src/tui/utils/providerSettingsModeMapper.ts` | Input type `HookMode`, output `PanelMode` |
| `src/tui/hooks/useProviderSettingsInput.ts` | No change needed (gets mode from hook, already typed) |
| `src/test-helpers/provider-settings-state-fixtures.ts` | Import `HookMode` instead of local `SettingsViewMode` |
| `src/tui/__tests__/provider-settings-oauth-guards.test.ts` | Remove `as PanelMode` casts, use `HookMode` |
| `src/tui/__tests__/provider-settings-mode-types.test.ts` | Import from production, not fixtures |

---

## 🟡 FIX 2: DRY Violation in deleteConfirmModeHandler.ts

### Problem

Three near-identical confirmation blocks:

```typescript
if (mode.type === 'delete-profile') {
  if (input === 'y' || input === 'Y') {
    void providerSettings.removeProfile(mode.providerId, mode.profileName).then(() => {
      providerSettings.setMode({ type: 'list' });
    });
    return true;
  }
  if (key.escape || input === 'n' || input === 'N') {
    providerSettings.setMode({ type: 'list' });
    return true;
  }
  return true;
}
// IDENTICAL pattern repeated for delete-api-key and disconnect-oauth
```

### Fix

Extract a generic `handleConfirmation` helper:

```typescript
function handleConfirmation(
  input: string,
  key: Key,
  onConfirm: () => Promise<void>,
  onCancel: () => void
): boolean {
  if (input === 'y' || input === 'Y') {
    void onConfirm().then(onCancel);
    return true;
  }
  if (key.escape || input === 'n' || input === 'N') {
    onCancel();
    return true;
  }
  return true; // Consume all input in confirmation mode
}
```

Then the handler becomes:

```typescript
export function handleDeleteConfirmMode(mode, input, key, ps): boolean {
  const cancel = () => ps.setMode({ type: 'list' });

  if (mode.type === 'delete-profile') {
    return handleConfirmation(input, key, () => ps.removeProfile(mode.providerId, mode.profileName), cancel);
  }
  if (mode.type === 'delete-api-key') {
    return handleConfirmation(input, key, () => ps.removeApiKey(mode.providerId), cancel);
  }
  if (mode.type === 'disconnect-oauth') {
    return handleConfirmation(input, key, () => ps.disconnectOauth(mode.providerId), cancel);
  }
  return false;
}
```

### Files to change

| File | Change |
|------|--------|
| `src/tui/inputHandlers/deleteConfirmModeHandler.ts` | Extract `handleConfirmation`, reduce 3 blocks to 3 one-liners |

---

## 🟡 FIX 3: Remove Undocumented 'r' Keybind

### Problem

`listModeHandler.ts` lines 168-171:

```typescript
// 'r' to refresh
if (input === 'r' || input === 'R') {
  void providerSettings.reload();
}
```

Rule 19 from the example map states: *"Complete keybind table: Enter and d are the only action keybinds."* The feature file scenario "Footer updates based on selected item type" lists only Enter, d, /, Tab, and Esc. No 'r'.

The tests verify e/n/t do nothing but there is no test for 'r'.

### Fix

1. **Delete** the `'r' to refresh` block from `listModeHandler.ts`
2. Provider settings already reload on mount and after mutations — manual refresh is unnecessary

### Files to change

| File | Change |
|------|--------|
| `src/tui/inputHandlers/listModeHandler.ts` | Remove lines 168-171 ('r' handler) |

---

## 🟡 FIX 4: Confirmation Dialog Wording Mismatch

### Problem

Feature file says:
- Scenario 20: `Then a confirmation dialog appears: "Delete profile work-vllm? (y/n)"`

Panel renders (line 181):
- `Are you sure you want to delete profile "work-vllm"?`

Feature file says:
- Scenario 15: `Then a confirmation dialog appears: "Delete API key for Google Gemini? (y/n)"`

Panel renders (line 208):
- `Delete API key for Google Gemini? (y/n)` ← This one matches ✓

Feature file says:
- Scenario 18: `Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"`

Panel renders (line 236):
- `Disconnect Claude OAuth? (y/n)` ← This one matches ✓

Only the **profile delete** dialog wording is wrong.

### Fix

Change `ProviderSettingsPanel.tsx` line 181 from:
```
Are you sure you want to delete profile "{mode.profileName}"?
```
to:
```
Delete profile {mode.profileName}? (y/n)
```

### Files to change

| File | Change |
|------|--------|
| `src/tui/components/ProviderSettingsPanel.tsx` | Line 181 — match feature file wording |

---

## 🟢 FIX 5: Stale Comment — "All 19 supported rig providers"

### Problem

`src/utils/provider-config.ts` line 75:
```typescript
/**
 * All 19 supported rig providers
 */
export const SUPPORTED_PROVIDERS = [
```

There are 16 providers now, not 19.

### Fix

Change comment to `All 16 supported providers` (or just `Supported providers`).

### Files to change

| File | Change |
|------|--------|
| `src/utils/provider-config.ts` | Line 75 comment |

---

## 🟢 FIX 6: Weak saveProfile Success Test

### Problem

Test "saveProfile accepts OpenAI API provider" (line 476-493) uses a try/catch that only asserts the guard message is absent. If `saveProfile` throws for a filesystem error, the test passes silently.

```typescript
try {
  await saveProfile('openai', 'test-profile', { ... });
} catch (err) {
  expect((err as Error).message).not.toContain('Profiles are only supported');
}
```

### Fix

Replace with a proper mock + assertion:

```typescript
// Mock writeFile to verify saveProfile doesn't throw the guard error
// and actually attempts to write
await expect(
  saveProfile('openai', 'test-profile', { baseUrl: 'http://localhost:8080', apiKey: 'test' })
).resolves.not.toThrow();
```

Or mock `fs.writeFile` and assert it was called. The point is: verify the happy path actually executes, not just that a specific error message is absent.

### Files to change

| File | Change |
|------|--------|
| `src/tui/__tests__/provider-settings-oauth-guards.test.ts` | Scenario 9 test — stronger assertion |

---

## Summary: Execution Order

1. **FIX 1** (Type split) — most impactful, touches the most files, creates the new type file
2. **FIX 2** (DRY confirmation) — independent, touches 1 file
3. **FIX 3** (Remove 'r') — 1 line delete
4. **FIX 4** (Dialog wording) — 1 line change
5. **FIX 5** (Stale comment) — 1 line change
6. **FIX 6** (Test assertion) — 1 test change

FIX 1 and FIX 2 can be done in parallel since they touch different files (except both touch `deleteConfirmModeHandler.ts` — do FIX 1 first, then FIX 2).
