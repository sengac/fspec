# PROV-031 — Model Screen Analysis

**Date:** 2026-03-03  
**Parent:** PROV-029  
**Screenshot:** `verybroken.png`

---

## What the screen looks like (broken state)

```
Select Model (27 items)

▶ 📁 openai: qwen3-coder-next (unreachable) (0 models)  (unreachable)
▼ 📁 anthropic: test-profile (unreachable) (0 models)   (unreachable)
▶ 📁 gemini: test-profile (unreachable) (0 models)       (unreachable)
▶ Google (21 models)
▶ Z.AI (9 models)
▶ OpenAI (19 models)
▼ Anthropic (19 models)
  > claude-sonnet-4-6
    claude-opus-4-6   [R] [V] [200k]
    ...
▶ Codex (ChatGPT) (8 models)

Enter: select | ←→: collapse/expand | r: refresh | Tab: settings | / filter | Esc: close
[R] Reasoning | [V] Vision | 📁 Profile (local server)
```

---

## Issue 1 — Stale profile sections for non-OpenAI providers

### What you see
Three profile sections appear at the top of the model list for `anthropic` and `gemini` providers. All are unreachable with 0 models.

### Root cause
`loadProfileSections()` in `src/tui/services/modelInitializationService.ts` (line 247) iterates **all** `SUPPORTED_PROVIDERS`:

```typescript
// BROKEN — iterates ALL providers
for (const providerId of SUPPORTED_PROVIDERS) {
  const profiles = await loadProviderProfiles(providerId);
  ...
}
```

`SUPPORTED_PROVIDERS` includes `anthropic`, `gemini`, `openai`, `codex`, etc. Since the user config (`~/.fspec/fspec-config.json`) has stale profiles under `anthropic` (the `localhost:8888` entry created during OAuth development — documented in PROV-029 description) and `gemini`, these get loaded and become profile sections.

PROV-029 Rule [21] established that **profiles are only for OpenAI API provider**, and `saveProfile()` in `provider-config.ts` already enforces this guard. But `loadProfileSections()` was never updated to match.

### Fix
```typescript
// CORRECT — only load profiles for openai
for (const providerId of ['openai'] as const) {
  const profiles = await loadProviderProfiles(providerId);
  ...
}
```

**File:** `src/tui/services/modelInitializationService.ts` ~line 247

---

## Issue 2 — Unreachable + 0 models sections shown

### What you see
`openai: qwen3-coder-next (unreachable) (0 models)` appears in the list. When expanded it shows nothing. It's navigable clutter.

### Root cause
After Fix 1 narrows profile loading to `openai` only, any OpenAI API profile pointing at an offline server still generates a section with `isUnreachable: true` and `models: []`. The section is silently added to the list — there is no post-load filter.

### Fix
In `initializeModels()`, filter the combined sections array after building it:

```typescript
// After combining profile + cloud sections:
const sections = [...profileSections, ...cloudSections].filter(
  s => !s.isUnreachable || s.models.length > 0
);
```

**Logic:** Keep unreachable sections only if they have models (partial server failure — some models loaded). Drop silently if both unreachable AND empty.

**File:** `src/tui/services/modelInitializationService.ts` in `initializeModels()` ~line 416

---

## Issue 3 — Footer says "Tab: settings" instead of "Tab: Switch to providers"

### What you see
```
Enter: select | ←→: collapse/expand | r: refresh | Tab: settings | / filter | Esc: close
```

### Root cause
PROV-029 Rule [12] required symmetric Tab hint labels:
- Provider settings panel → "Tab: Switch to models" ✅ (already fixed in `ProviderSettingsPanel.tsx`)
- Model selector → "Tab: Switch to providers" ❌ (never updated)

`ModelSelectorView.tsx` line 271 still has the old text.

### Fix
```tsx
// BEFORE
Enter: select | ←→: collapse/expand | r: refresh | Tab: settings | / filter | Esc: close

// AFTER
Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close
```

**File:** `src/tui/components/ModelSelectorView.tsx` line 271

---

## Issue 4 — "(27 items)" mixes section headers with model rows

### What you see
```
Select Model (27 items)
```

### Root cause
`ModelSelectorView.tsx` line 143 renders:
```tsx
<Text dimColor> ({flatItems.length} items)</Text>
```

`flatItems` is the **flat navigation list** — it includes both section header rows and individual model rows. When Anthropic is expanded with 19 models, the count includes:
- 3 wrong profile section headers (from Issue 1)
- 1 Anthropic section header
- 19 Anthropic model rows
- 4 other provider section headers (Google, Z.AI, OpenAI, Codex)

Total = 27. After fixing Issue 1 it becomes 24. But it's still wrong — a user cares about how many **models** they can select, not how many navigation rows exist.

### Fix
```tsx
// BEFORE
<Text dimColor> ({flatItems.length} items)</Text>

// AFTER
<Text dimColor> ({flatItems.filter(i => i.type === 'model').length} models)</Text>
```

**File:** `src/tui/components/ModelSelectorView.tsx` line 143

---

## What the screen should look like (fixed state)

```
Select Model (76 models)

▶ 📁 openai: work-vllm (5 models)        ← only if reachable with models
▶ Google (21 models)
▶ Z.AI (9 models)
▶ OpenAI (19 models)
▼ Anthropic (19 models)
  > claude-sonnet-4-6
    claude-opus-4-6   [R] [V] [200k]
    ...
▶ Codex (ChatGPT) (8 models)

Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close
[R] Reasoning | [V] Vision | 📁 Profile (local server)
```

---

## Files to change

| File | Change |
|------|--------|
| `src/tui/services/modelInitializationService.ts` | Fix `loadProfileSections()` loop + add unreachable filter in `initializeModels()` |
| `src/tui/components/ModelSelectorView.tsx` | Fix footer text + fix item count label |

Total: **2 files, 4 line changes.**
