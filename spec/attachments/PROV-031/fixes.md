# PROV-031 Post-Implementation Review — Required Fixes

**Story:** Model Screen — stale profile sections from non-OpenAI providers, footer text, unreachable section filtering
**Status at review:** validating
**Reviewer:** Claude Code
**Date:** 2026-03-03 (updated after second review pass)

---

## Review History

| Pass | Date | Outcome |
|------|------|---------|
| Pass 1 (original) | 2026-03-03 | 7 findings (FIX-1 through FIX-7). Rewrites required for Scenarios 4 & 5. |
| Pass 2 (this file) | 2026-03-03 | FIX-1 through FIX-7 all resolved. 3 new findings from second pass. |

---

## Pass 2 — Current Status

All 5 scenario tests pass (5/5, 100% coverage). All Pass 1 findings were addressed:

| Fix | Status | Notes |
|-----|--------|-------|
| FIX-1: 6 dead imports in `useModelSelectorState.ts` | ✅ Done | ESLint clean, `SUPPORTED_PROVIDERS` import removed |
| FIX-2: Scenarios 4 & 5 rewritten to `render()` + `lastFrame()` | ✅ Done | File moved to `components/__tests__/`, uses `ink-testing-library` |
| FIX-3: `extractModelIdForRegistry` duplication resolved | ✅ Done | Hook imports from service; AgentView function renamed `normalizeModelIdForMatch` with documented intentional divergence |
| FIX-4: Test file moved to `components/__tests__/` with `.tsx` | ✅ Done | `modelScreenProv031.test.tsx` |
| FIX-5: Full footer string asserted | ✅ Done | `expect(frame).toContain('r: refresh \| Tab: Switch to providers \| / filter \| Esc: close')` |
| FIX-6: Integration test `ModelSelectorScreen.integration.test.tsx:801` coverage | ⚠️ Not re-linked | Still mapped to `model-selector-screen.feature` (TUI-073). Low priority — it tests the same behaviour from a different angle. |
| FIX-7: `buildFlatItems` populates `section.models` | ✅ Done | `section.models` is populated with the same model objects added to the flat list |

---

## Pass 2 — New Findings

---

### 🔴 NEW-1 · Dead Function `findModelInSections` in `modelInitializationService.ts`

**File:** `src/tui/services/modelInitializationService.ts` lines 316–338

ESLint reports it directly: `'findModelInSections' is defined but never used`.

```typescript
// Lines 316–338 — never called from anywhere
function findModelInSections(
  sections: ProviderSection[],
  providerId: string,
  modelId: string
): { section: ProviderSection; model: NapiModelInfo } | null {
  const section = sections.find(s => s.providerId === providerId);
  if (!section || !section.hasCredentials) {
    return null;
  }
  const normalizedModelId = extractModelIdForRegistry(modelId);
  const model = section.models.find(
    m => extractModelIdForRegistry(m.id) === normalizedModelId
  );
  if (!model) {
    return null;
  }
  return { section, model };
}
```

This function was superseded by `findSectionForPersistedModel` from `../utils/model-selection`
during BUG-097 work. The old function was never removed. 21 lines of dead code sitting in
the model initialization service, contributing to its length and cluttering the call graph.

**Action:** Delete `findModelInSections` (lines 316–338) in its entirety. Run ESLint to confirm
the warning disappears.

---

### 🔴 NEW-2 · Dead Prop `sections` in `ModelSelectorViewProps` — Interface Lying About its Contract

**File:** `src/tui/components/ModelSelectorView.tsx` lines 30–31

The interface declares `sections: ProviderSection[]` with a JSDoc comment "for stats display":

```typescript
export interface ModelSelectorViewProps {
  /** Provider sections (for stats display) */
  sections: ProviderSection[];   // ← declared but never used
  /** Flattened list of items to render */
  flatItems: ModelSelectorItem[];
  // ...
}
```

The component function never destructures `sections`:

```typescript
export function ModelSelectorView({
  width, height, flatItems,           // ← sections is absent here
  selectedSectionIdx, selectedModelIdx,
  // ...
}: ModelSelectorViewProps): React.ReactElement {
```

`sections` is accepted silently (TypeScript ignores extra destructuring) and discarded.

**Impact:**
- `ModelSelectorScreen.tsx` line 247 passes `sections={providerSections}` — wasted allocation
- Both PROV-031 tests pass `sections={[]}` — meaningless prop
- The interface documents a contract the component does not honour
- A developer reading the interface would assume stats display exists — it doesn't

The stat display was likely planned but replaced with the inline `flatItems.filter(i => i.type === 'model').length` approach. The `sections` prop was never wired in.

**Action:**
1. Remove `sections` from `ModelSelectorViewProps` interface
2. Remove `sections={providerSections}` from `ModelSelectorScreen.tsx` line 247
3. Remove `sections={[]}` from both `render()` calls in `modelScreenProv031.test.tsx`
4. Run `npm run build` — TypeScript will confirm all callers are updated

---

### 🟡 NEW-3 · Prettier Formatting Failures on `ModelSelectorView.tsx` and `ModelSelectorScreen.tsx`

Running `npm run format` (or `npx prettier --check`) fails on two files changed by this story:

#### `ModelSelectorView.tsx` — 4 errors

```
17:14  imports — multi-line when Prettier wants inline:
       Replace `⏎··ProviderSection,⏎··ModelSelectorItem,⏎` with `·ProviderSection,·ModelSelectorItem·`

223:69  [V] badge JSX — split across lines incorrectly

250:30  Scrollbar thumb math expression — line break inside arithmetic expression

271:78  Footer text — Prettier wants line break BEFORE "providers", not at "/" (see below)
```

The footer issue at line 271 is the most important. The current source is:

```jsx
<Text dimColor>
  Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | /
  filter | Esc: close
</Text>
```

Prettier breaks at a different column. This also creates a fragile reliance on JSX whitespace
collapsing `| /\n  filter` → `| / filter`. The test passes because Ink collapses it correctly,
but it is brittle and format-unstable.

**Fix:** Use an explicit string expression to make it Prettier-stable and whitespace-safe:

```jsx
<Text dimColor>
  {'Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close'}
</Text>
```

#### `ModelSelectorScreen.tsx` — ~50 errors (pre-existing, not introduced by PROV-031)

The `for` loop on line 96 and the `useInput` callback block (lines 122–240) have indentation
that diverges from Prettier's 2-space rule. These appear to be pre-existing violations not
introduced by PROV-031 (the story's diff does not touch those lines). Confirm with `git diff`
before fixing — if pre-existing, open a separate clean-up card rather than attributing to PROV-031.

**Action:**
1. Fix `ModelSelectorView.tsx` formatting — run `npx prettier --write src/tui/components/ModelSelectorView.tsx` and replace the footer `<Text>` with the explicit string expression above
2. Confirm `ModelSelectorScreen.tsx` violations are pre-existing (git blame); if so, exclude from this story's scope and file separately

---

## FIX-6 Follow-Up — Integration Test Coverage Gap Still Open

`ModelSelectorScreen.integration.test.tsx:801-833` contains the best behavioural test for
Scenario 2 (unreachable + 0 models), using full `render()` → `waitForModelsLoaded()` → `lastFrame()`.
It is still mapped only to `model-selector-screen.feature` (TUI-073 scope).

The PROV-031 Scenario 2 coverage points to `modelScreenProv031.test.tsx:238-283` (a service
unit test). Both cover the same rule from different angles, which is fine. But the integration
test provides stronger end-to-end confidence and should also be linked here.

**Action (low priority):** Run:
```bash
fspec link-coverage \
  "model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering" \
  --scenario "Unreachable OpenAI profile with zero models is filtered from the model screen" \
  --testFile "src/tui/components/__tests__/ModelSelectorScreen.integration.test.tsx" \
  --testLines "801-833"
```

---

## Updated Implementation Checklist

### Pass 1 Items (all resolved ✅)
- [x] FIX-1: Remove 6 dead imports from `useModelSelectorState.ts`
- [x] FIX-2: Rewrite Scenarios 4 & 5 tests to use `render()` + `lastFrame()`
- [x] FIX-3: Import `extractModelIdForRegistry` from service in hook; document AgentView divergence
- [x] FIX-4: Move test file to `components/__tests__/` with `.tsx` extension
- [x] FIX-5: Assert complete footer string
- [x] FIX-7: Fix `buildFlatItems` to populate `section.models` consistently

### Pass 2 Items (outstanding)
- [ ] **NEW-1:** Delete `findModelInSections` (lines 316–338) from `modelInitializationService.ts`
- [ ] **NEW-2:** Remove dead `sections` prop from `ModelSelectorViewProps`, `ModelSelectorScreen`, and tests
- [ ] **NEW-3:** Fix `ModelSelectorView.tsx` Prettier violations (4 errors); use explicit string for footer
- [ ] **NEW-3:** Confirm `ModelSelectorScreen.tsx` Prettier violations are pre-existing; file separately if so
- [ ] **FIX-6:** Re-link integration test coverage for unreachable server scenario (low priority)
- [ ] Run `npm test` — all 5 scenarios must still pass after all fixes
- [ ] Run `npx eslint src/tui/services/modelInitializationService.ts` — 0 warnings after NEW-1
- [ ] Run `npx prettier --check src/tui/components/ModelSelectorView.tsx` — 0 errors after NEW-3
