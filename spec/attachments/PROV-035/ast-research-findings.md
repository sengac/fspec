# PROV-035: AST Research Findings — OAuth Status Echo Replacement

## Summary

Replace the redundant `oauth-status` child row with an actionable "Logout from OAuth [Claude/ChatGPT]" line. Make Enter trigger the disconnect-oauth confirmation dialog (currently only `d` works). Keep `d` for backward compat but update footer to advertise Enter as primary.

---

## 1. Source Files That Need Changes

### 1.1 `src/tui/hooks/useProviderSettingsState.ts` — `buildNavItems()` (L155-160)

**Current behavior:** Creates the `oauth-status` nav item with label `✓ OAuth [Claude]` / `✓ OAuth [ChatGPT]`.

```typescript
// Line 155-160
if (isOAuthProvider(provider.id) && provider.hasOAuthTokens) {
  items.push({
    type: 'oauth-status',
    providerId: provider.id,
    label: `✓ OAuth [${provider.status?.source || provider.name}]`,
  });
}
```

**Change:** Update label to `Logout from OAuth [Claude]` / `Logout from OAuth [ChatGPT]`.

```typescript
label: `Logout from OAuth [${provider.status?.source || provider.name}]`,
```

No structural changes needed — the `oauth-status` nav item type, `providerId`, and conditional logic are all correct.

---

### 1.2 `src/tui/inputHandlers/listModeHandler.ts` — `handleActions()` (L118-143)

**Current behavior:** The `key.return` block handles `provider`, `oauth-login`, `api-key`, `profile`, and `add-profile` — but **NOT** `oauth-status`. Pressing Enter on oauth-status does nothing.

```typescript
// Line 118-143
if (key.return) {
  if (currentItem.type === 'provider') { ... }
  else if (currentItem.type === 'oauth-login') { ... }
  else if (currentItem.type === 'api-key') { ... }
  else if (currentItem.type === 'profile' && currentProfile) { ... }
  else if (currentItem.type === 'add-profile') { ... }
  // ← oauth-status NOT handled!
  return;
}
```

**Change:** Add `oauth-status` case to the `key.return` block, triggering disconnect-oauth mode:

```typescript
} else if (currentItem.type === 'oauth-status') {
  providerSettings.setMode({
    type: 'disconnect-oauth',
    providerId: currentItem.providerId,
  });
}
```

**The `d` key handler (L147-166) should remain unchanged** — it already handles `oauth-status` → `disconnect-oauth`. This provides backward compat.

---

### 1.3 `src/tui/utils/providerSettingsHelpers.ts` — `getFooterHints()` (L20-21)

**Current behavior:**

```typescript
case 'oauth-status':
  return `d: disconnect · ${FOOTER_COMMON}`;
```

**Change:** Update to advertise Enter as primary action:

```typescript
case 'oauth-status':
  return `Enter: logout · ${FOOTER_COMMON}`;
```

We drop `d: disconnect` from the footer since Enter is now the primary action and the line text itself says "Logout from OAuth". The `d` key still works but doesn't need to be advertised.

---

### 1.4 `src/tui/components/ProviderSettingsPanel.tsx` — oauth-status renderer (L666-683)

**Current behavior:** Renders in green with the label text (e.g. `✓ OAuth [Claude]`).

```tsx
if (item.type === 'oauth-status') {
  return (
    <Box key={`oauth-status-${item.providerId}`} width={contentWidth}>
      <Text
        backgroundColor={isSelected ? 'green' : undefined}
        color={isSelected ? 'black' : 'green'}
        wrap="truncate"
      >
        {isSelected ? '> ' : '  '}
        {'    '}{item.label}
      </Text>
    </Box>
  );
}
```

**Change:** The label now reads "Logout from OAuth [Claude]" which self-describes the action. Consider changing the color from green (connected status) to red/magenta (destructive action) to signal it's an action, not a status display:

```tsx
<Text
  backgroundColor={isSelected ? 'red' : undefined}
  color={isSelected ? 'black' : 'red'}
  wrap="truncate"
>
```

However, this is a UX judgment call — green with the "Logout" text may be sufficient since the text itself is clear. The color change is optional.

---

## 2. Additional Bug Found: API Key Row Shows OAuth Status

### Problem

When OAuth is active, `reload()` in `useProviderSettingsState.ts` (L282-313) **overwrites the entire provider `status` object** with OAuth info:

```typescript
// Line 290-296 (Anthropic)
status = {
  hasKey: true,
  maskedKey: 'OAuth',
  source: 'Claude',
};
```

The original API key information (if the user also has `ANTHROPIC_API_KEY` set) is lost. The `api-key` row renderer (L686-718) reads `provider?.status` which is now the OAuth-overwritten status. This is why the screenshot shows:

```
🔑 API key ✓ OAuth [Claude]
```

Instead of one of:
- `🔑 API key ✓ sk-ant-••••Qr7K [env]` (if API key exists)
- `🔑 API key (not set)` (if no API key)

**This is OUT OF SCOPE for PROV-035** but should be tracked as a separate bug. The fix would involve preserving the original API key status separately from the OAuth status, or storing both on the `ProviderDisplayInfo`.

---

## 3. Test Files That Need Updates

### 3.1 `src/tui/__tests__/provider-settings-oauth-guards.test.ts`

**Line 700-704 — OAuth status item fixture:**
```typescript
const item: SettingsNavItem = {
  type: 'oauth-status',
  providerId: 'anthropic',
  label: '✓ OAuth [Claude]',  // ← Update to 'Logout from OAuth [Claude]'
};
```

**Line 854 — Footer hint assertion:**
```typescript
expect(getFooterHints('oauth-status')).toBe(
  'd: disconnect · / filter · Tab: Switch to models · Esc: close'
  // ← Update to 'Enter: logout · / filter · Tab: Switch to models · Esc: close'
);
```

**Line 882 — All-footer-variants assertion:**
The `oauth-status` entry in the `itemTypes` loop is tested for `Tab: Switch to models` — this assertion still holds and needs no change.

**NEW TEST needed: Enter on oauth-status triggers disconnect-oauth mode** — mirror the existing 'd' key test at L698-718.

---

### 3.2 `src/tui/inputHandlers/__tests__/anthropic-oauth-tui.test.ts`

**Line 497-526 — Existing `d` key test:**
```typescript
it('should set disconnect-oauth mode when pressing "d" on OAuth status item', () => {
  const currentItem: SettingsNavItem = {
    type: 'oauth-status',
    providerId: 'anthropic',
    label: '✓ OAuth [Claude]',  // ← Update to 'Logout from OAuth [Claude]'
  };
  // ... rest stays the same, d key should still work
});
```

**NEW TEST needed:** `it('should set disconnect-oauth mode when pressing Enter on OAuth status item', ...)`

---

### 3.3 `src/tui/inputHandlers/__tests__/listModeHandler-codex-oauth.test.ts`

No direct oauth-status fixtures in this file, but the comment at L178 references `oauth-status item` — no code change needed, just ensure existing tests still pass.

---

### 3.4 `src/tui/inputHandlers/__tests__/anthropic-parity-regression.test.ts`

**Line 187-216 — Tests that `d` on provider row does nothing:**
These tests remain valid (d on **provider** rows still does nothing). No changes needed.

---

### 3.5 `src/tui/__tests__/oauth-tui-broken-flows.test.ts`

**Line 82-89 — Label assertion for Codex:**
```typescript
expect(statusItems[0].label).toContain('✓ OAuth [ChatGPT]');
// ← Update to expect 'Logout from OAuth [ChatGPT]'
```

**Line 106-124 — Anthropic status item assertions:**
Status items are filtered by `type: 'oauth-status'` — these still work. But if any label assertion exists, update it.

---

### 3.6 `src/tui/components/__tests__/ProviderSettingsScreen.integration.test.tsx`

**Line 503 comment:**
```typescript
// Anthropic (OAuth) expanded items: oauth-status, oauth-login (browser), oauth-login (headless), api-key
```

Comment-only, but the item count (4 down-presses to reach API key) remains correct since we're not adding or removing items.

---

### 3.7 `src/test-helpers/provider-settings-state-fixtures.ts`

Check if this contains any hardcoded `oauth-status` labels that need updating.

---

### 3.8 `src/tui/__tests__/fixtures/oauthTestFixtures.ts`

No `oauth-status` label strings in this file — it only builds `ProviderDisplayInfo` objects. No changes needed.

---

## 4. Feature Files That Need Updates

### 4.1 `spec/features/provider-settings-oauth-guards.feature`

**Line 42-43 — Scenario: Expanding an OAuth provider:**
```gherkin
| ✓ OAuth [Claude]                   |
```
Update to:
```gherkin
| Logout from OAuth [Claude]         |
```

**Line 142-145 — Scenario: Pressing 'd' on an OAuth status item:**
```gherkin
Scenario: Pressing 'd' on an OAuth status item shows disconnect confirmation
  Given I have the cursor on "✓ OAuth [Claude]"
```
Update label text and consider adding/modifying scenario for Enter key too.

**Line 178-179 — Footer expectations:**
```gherkin
| oauth status       | d: disconnect · / filter · Tab: Switch to models · Esc: close           |
```
Update to:
```gherkin
| oauth status       | Enter: logout · / filter · Tab: Switch to models · Esc: close           |
```

**NEW SCENARIO needed:** "Enter on OAuth status item shows disconnect confirmation"

---

### 4.2 `spec/features/oauth-tui-broken-flows.feature`

Check for any `✓ OAuth [Claude]` or `✓ OAuth [ChatGPT]` label references that need updating.

---

## 5. Type Definitions — No Changes Needed

- `SettingsNavItem` type (`ProviderSettingsPanel.tsx` L104-119) — `oauth-status` variant's shape (`type`, `providerId`, `label`) doesn't change
- `HookMode` type (`settingsMode.ts` L18-35) — `disconnect-oauth` already exists
- `PanelMode` type (`ProviderSettingsPanel.tsx` L59-99) — `disconnect-oauth` already exists
- `providerSettingsModeMapper.ts` — `disconnect-oauth` passthrough already works

---

## 6. Disconnect Confirmation Dialog — No Changes Needed

`ProviderSettingsPanel.tsx` L219-245 renders the disconnect-oauth confirmation. This is triggered by setting `mode.type === 'disconnect-oauth'` — which both `d` and the new Enter handler will do. The dialog text ("Disconnect Claude/ChatGPT OAuth? (y/n)") is independent of how we got there. No changes needed.

`deleteConfirmModeHandler.ts` L64-70 handles the y/n confirmation for `disconnect-oauth`. No changes needed.

---

## 7. Change Summary

| File | Change | Severity |
|------|--------|----------|
| `useProviderSettingsState.ts` L159 | Change label from `✓ OAuth [...]` to `Logout from OAuth [...]` | Required |
| `listModeHandler.ts` L118-143 | Add `oauth-status` case to `key.return` block | Required |
| `providerSettingsHelpers.ts` L20-21 | Change footer from `d: disconnect` to `Enter: logout` | Required |
| `ProviderSettingsPanel.tsx` L666-683 | Optional: change color from green to red for logout affordance | Optional |
| `provider-settings-oauth-guards.test.ts` | Update label fixtures, footer assertions, add Enter test | Required |
| `anthropic-oauth-tui.test.ts` | Update label fixture, add Enter key test | Required |
| `oauth-tui-broken-flows.test.ts` | Update label assertions | Required |
| `provider-settings-oauth-guards.feature` | Update label text, footer text, add Enter scenario | Required |
| `oauth-tui-broken-flows.feature` | Update label text if referenced | Check |

**Estimated effort:** 2 story points — small, well-scoped, all mechanical changes.
