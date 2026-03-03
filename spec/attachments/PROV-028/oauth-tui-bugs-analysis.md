# OAuth TUI Bugs Analysis — PROV-011 & PROV-012

Comprehensive analysis of all issues found in the Anthropic and Codex OAuth TUI integration, discovered via visual testing (screenshots) and code review.

---

## 🔴 BUG 1: Claude Browser OAuth Opens Wrong URL — Code Entry Form Never Shown

**Severity:** Critical — the "Login with Claude (browser)" flow is completely broken  
**Affected cards:** PROV-021 (browser server), PROV-025 (TUI integration)  
**Screenshots:** `wrong3.png`

### Problem

The Claude browser OAuth flow opens the Anthropic authorization URL directly in the browser instead of opening the local form page. This means the user:

1. Gets redirected to Anthropic's auth page → authorizes → lands on `platform.claude.com/oauth/code/callback` 
2. Sees "Authentication Code — Paste this into Claude Code:" with a code
3. But the **local form page** at `http://localhost:{port}/` (which has the actual paste input) **was never shown** in the browser
4. The TUI meanwhile shows only "Waiting for authorization..." with no code input field
5. **The flow is dead** — the user has the code but nowhere to enter it

### Root Cause

In `codelet/providers/src/claude_oauth_server.rs` line 118:

```rust
// 3. Open browser (skipped in tests)
if config.open_browser {
    if let Err(e) = open::that(&auth_url) {   // ← WRONG: opens Anthropic's authorize URL
```

This opens the browser to `https://claude.ai/oauth/authorize?client_id=...` (the Anthropic auth URL). It should open `http://localhost:{port}/` which serves the form page built by `build_form_html()`. That form page:
- Links to the auth URL (opens in new tab via `target="_blank"`)
- Has a text input for pasting `code#state`
- POSTs to `/submit` on the local server

### Fix

Change line 118 from:
```rust
if let Err(e) = open::that(&auth_url) {
```
to:
```rust
let form_url = format!("http://localhost:{port}/");
if let Err(e) = open::that(&form_url) {
```

The form page already has the auth URL as a clickable link with `target="_blank"`. The flow becomes:
1. Browser opens to `http://localhost:{port}/` → user sees the form page
2. User clicks the auth URL link → new tab opens to Anthropic's auth page
3. User authorizes → Anthropic shows the code on callback page
4. User copies code, goes back to local form tab, pastes code, clicks Submit
5. Local server exchanges tokens → success

### Key Files
- `codelet/providers/src/claude_oauth_server.rs` — lines 79-123 (entry point + browser open)
- `codelet/providers/src/claude_oauth_server.rs` — lines 384-425 (`build_form_html` — the form page that should be shown)
- `src/tui/components/ProviderSettingsPanel.tsx` — lines 295-321 (`oauth-browser-waiting` render — only shows spinner, no input)
- `src/tui/hooks/useProviderSettingsState.ts` — lines 493-523 (`startBrowserLogin` — calls NAPI binding)

---

## 🔴 BUG 2: Expanded Codex Provider Shows Zero Sub-Items

**Severity:** Critical — authenticated Codex appears broken/empty when expanded  
**Affected cards:** PROV-017 (TUI OAuth Login Flow), PROV-019 (Codex integration bugs)  
**Screenshots:** `wrong4.png`

### Problem

When the Codex (ChatGPT) provider has OAuth tokens and the user expands it, the expanded view is **completely empty** — no login options, no profile info, no disconnect option, nothing.

Compare to Anthropic (screenshot `wrong2.png`) which correctly shows 3 sub-items when expanded:
- 🔑 Login with Claude (browser)
- 🔑 Login with Claude (headless)
- 📁 anthropic → http://localhost:8888

### Root Cause

In `src/tui/hooks/useProviderSettingsState.ts`, the `buildNavItems()` function (lines 130-195) has a logic gap:

```typescript
if (provider.isExpanded) {
  // OAuth login options ONLY shown when no tokens exist
  if (isOAuthProvider(provider.id) && !provider.hasOAuthTokens) {
    // push login items...
  }

  // Profiles pushed for each provider
  for (const profile of provider.profiles) {
    items.push({ type: 'profile', ... });
  }

  // "Create Profile" ONLY for non-OAuth providers
  if (!isOAuthProvider(provider.id)) {
    items.push({ type: 'add-profile', ... });
  }
}
```

When Codex `hasOAuthTokens === true`:
- Login options skipped (guarded by `!provider.hasOAuthTokens`)
- Profiles: Codex has 0 profiles (it uses OAuth, not profile-based config)
- "Create Profile" skipped (guarded by `!isOAuthProvider()`)

**Result: zero items pushed → empty expansion**

### Fix

When an OAuth provider has tokens and is expanded, it should show:
1. A status/info item showing the auth method and status (e.g., "✓ OAuth [ChatGPT]")
2. Optionally: a "Disconnect" action item, OR rely on the existing `d` key shortcut being visible in the footer

At minimum, the status indicator issue (BUG 3 below) should be fixed so expanding doesn't hide auth info.

### Key Files
- `src/tui/hooks/useProviderSettingsState.ts` — lines 130-195 (`buildNavItems()` function)
- `src/tui/components/ProviderSettingsPanel.tsx` — lines 478-538 (provider row rendering)

---

## 🟡 BUG 3: Codex Status Indicator Disappears When Expanded

**Severity:** Medium — confusing UX, auth state not visible  
**Affected cards:** PROV-017 (TUI OAuth Login Flow)  
**Screenshots:** `wrong2.png` (collapsed with status) vs `wrong4.png` (expanded without status)

### Problem

When collapsed, Codex shows: `▶ Codex (ChatGPT) ✓ OAuth [ChatGPT]`  
When expanded, Codex shows: `▼ Codex (ChatGPT)` — the status text is **gone**

The Anthropic provider correctly shows status when expanded: `▼ Anthropic ✓ sk-ant-•••••••VgAA [file] (1 profile)`

### Root Cause

The provider row rendering in `ProviderSettingsPanel.tsx` lines 496-506 renders the status:

```tsx
{status?.hasKey ? (
  <Text color={isSelected ? 'black' : 'green'}>
    {' '} ✓ {status.maskedKey}
    {status.source && <Text dimColor={!isSelected}> [{status.source}]</Text>}
  </Text>
) : (
  <Text color={isSelected ? 'black' : 'gray'}> (not configured)</Text>
)}
```

This code should show status regardless of expanded state. The issue might be that when the Codex row is **selected** (highlighted with yellow background), the `isSelected ? 'black' : 'green'` coloring makes the green text invisible on the yellow background. But more likely, the status is being overwritten or the rendering is different when the row is the currently focused item in `wrong4.png`.

Actually, comparing screenshots carefully: in `wrong2.png` Codex is at the bottom and NOT selected (no `>` indicator). In `wrong4.png` Codex IS selected (`>` indicator, yellow background). The `isSelected ? 'black'` should render black text on yellow background, which should be visible. The status might be truncated off the right edge due to `wrap="truncate"` combined with the longer `▼ ` expanded prefix.

### Fix

Verify that the status text renders correctly when:
1. The provider row is selected (yellow background)
2. The provider is expanded (▼ prefix takes slightly more space with the expanded indicator)

May need to check `wrap="truncate"` behavior with long status strings.

### Key Files
- `src/tui/components/ProviderSettingsPanel.tsx` — lines 478-538 (provider row rendering)

---

## 🟡 BUG 4: PROV-012 Parent Still in "specifying" — All 8 Children Done

**Severity:** Medium — workflow tracking is incorrect  
**Affected cards:** PROV-012 (parent)

### Problem

PROV-012 is a parent/umbrella story with 8 child work units:
- PROV-020 (core OAuth) — ✅ done
- PROV-021 (browser server) — ✅ done
- PROV-022 (device/headless auth) — ✅ done
- PROV-023 (token refresh) — ✅ done
- PROV-024 (NAPI bindings) — ✅ done
- PROV-025 (TUI settings) — ✅ done
- PROV-026 (routing/model availability) — ✅ done
- PROV-027 (parity regression) — ✅ done

The parent PROV-012 is still in `specifying` status. It also has:
- No Example Mapping data (rules, examples, questions)
- No estimate
- No linked feature files

### Fix

After fixing the critical bugs (BUG 1), PROV-012 should be advanced through the workflow. The `review-fixes.md` attachment on PROV-012 already documented this (FIX 4).

---

## 🟡 BUG 5: PROV-011 Parent Still in "specifying" — All 5 Children Done

**Severity:** Medium — workflow tracking is incorrect  
**Affected cards:** PROV-011 (parent)

### Problem

PROV-011 is a parent/umbrella story with 5 child work units:
- PROV-013 (browser OAuth server) — ✅ done
- PROV-014 (device auth flow) — ✅ done
- PROV-015 (NAPI bindings) — ✅ done
- PROV-016 (custom fetch) — ✅ done
- PROV-017 (TUI OAuth flow) — ✅ done

The parent PROV-011 is still in `specifying` status.

### Fix

After fixing the critical bugs (BUG 2), PROV-011 should be advanced through the workflow.

---

## 🟡 BUG 6: Existing review-fixes.md on PROV-012 Not Addressed

**Severity:** Medium — known issues from previous review still open  
**Affected cards:** PROV-012

### Problem

The attachment `spec/attachments/PROV-012/review-fixes.md` documents 4 fixes that were never applied:
1. **FIX 1 (Blocker):** Parallel test failures — missing `#[serial]` annotations in `claude_oauth_routing_test.rs` and `claude_oauth_resolver_test.rs`
2. **FIX 2 (Low):** User-Agent version spec drift — rule says `2.1.2`, code says `2.1.3`
3. **FIX 3 (Low):** Feature file suggests `mcp_` prefixing is applied in production when it's only a parity reference function
4. **FIX 4 (Low):** Parent housekeeping — already captured in BUG 4 above

---

## 🟡 BUG 7: Headless Code Input Renders as Single Unwrapped Line — Pushes Layout Down

**Severity:** Medium — unusable code entry field for long tokens  
**Affected cards:** PROV-025 (TUI integration)

### Problem

When the user pastes an authorization code into the headless code entry field, the `codeInput` text renders as a single unwrapped line. OAuth codes are typically 80-200+ characters long. The raw `<Text>{mode.codeInput}</Text>` has no width constraint or wrapping, so it:

1. Extends horizontally far past the terminal width
2. Pushes subsequent UI elements (the "Enter to submit | Esc to cancel" hint) down off the visible area
3. The text may not even be visible since Ink's default `<Text>` doesn't wrap within a `<Box>`

### Root Cause

In `src/tui/components/ProviderSettingsPanel.tsx` lines 403-408:

```tsx
<Box marginTop={1}>
  <Text color="cyan">Code: </Text>
  <Text>
    {mode.codeInput}        {/* ← No width constraint, no wrap */}
    <Text inverse> </Text>  {/* cursor indicator */}
  </Text>
</Box>
```

The `<Text>` element has no `wrap` property and no constraining `<Box width={...}>` parent. Long pasted strings overflow the line.

### Fix

Wrap the code input in a width-constrained Box with `wrap="truncate-end"` or a scroll-style display showing only the last N characters. A common pattern for password/token entry:

```tsx
<Box marginTop={1} width={width - 6}>
  <Text color="cyan">Code: </Text>
  <Text wrap="truncate">
    {mode.codeInput}
    <Text inverse> </Text>
  </Text>
</Box>
```

Or show a masked/truncated view: display only the last ~40 characters with `...` prefix.

### Key Files
- `src/tui/components/ProviderSettingsPanel.tsx` — lines 403-408 (code input rendering)

---

## 🟡 BUG 8: Authorize URL Not Copyable — No Keybind, Not a Terminal Hyperlink

**Severity:** Medium — users can't visit the auth URL from headless mode  
**Affected cards:** PROV-025 (TUI integration)

### Problem

The headless login screen shows `Visit: https://claude.ai/oauth/authorize?...` as plain blue `<Text>`. Users cannot:

1. **Select the URL text** — TUI has no text selection mechanism
2. **Copy via keybind** — no `c` or `Ctrl+C` copy handler exists in `oauthModeHandler.ts`
3. **Click it** — the URL is not rendered as a terminal hyperlink (OSC 8 sequence)

The authorize URL is ~300-400 characters long (includes client_id, PKCE challenge, state, scopes, redirect_uri), making it impossible to manually type. Without a way to copy or click it, the headless flow is functionally broken.

### Fix

At minimum, add a keybind (e.g., `c` for "copy URL to clipboard") that uses a clipboard utility to copy `mode.authorizeUrl`. The hint text should update to show the keybind:

```
Enter to submit | c: copy URL | Esc to cancel
```

Better: render the URL as a terminal hyperlink using OSC 8 escape sequences, which most modern terminals (iTerm2, Alacritty, Wezterm, GNOME Terminal, Windows Terminal) support. This allows users to click/Cmd-click the link directly. Ink doesn't support this natively, but it can be done with raw ANSI:

```typescript
const hyperlink = `\u001b]8;;${url}\u0007${displayText}\u001b]8;;\u0007`;
```

Also: consider auto-opening the URL in a browser via `open` (the `open` npm package), with a keybind to retry/reopen if needed.

### Key Files
- `src/tui/components/ProviderSettingsPanel.tsx` — lines 396-399 (URL display)
- `src/tui/inputHandlers/oauthModeHandler.ts` — lines 38-67 (headless code entry input handling)

---

## 🔵 DESIGN: Browser and Headless Should Be a Single Unified Flow

**Severity:** Design improvement — not a bug per se, but a significant UX issue  
**Affected cards:** PROV-025 (Anthropic TUI), PROV-021 (browser server)

### Problem

The user currently sees two separate options:
- "Login with Claude (browser)" — opens local form page (when fixed), shows spinner in TUI
- "Login with Claude (headless)" — shows URL + code entry directly in TUI

But since Anthropic's redirect URI is **remote** (`console.anthropic.com/oauth/code/callback`), *both* flows require the user to manually copy a code. The distinction is only whether a browser is auto-opened. This is confusing because:

1. Users don't understand "headless" vs "browser" — they just want to log in
2. Both flows end up at the same place: user needs to paste `code#state`
3. The "browser" flow opens a local form page that duplicates what the TUI could do directly
4. Having two options doubles the maintenance surface

### Recommended Design

Merge into a single "Login with Claude" option that:
1. Auto-opens the auth URL in a browser (if available) — like the current browser flow
2. Simultaneously shows the URL in the TUI (as a copyable/clickable link) — like the current headless flow
3. Shows the code entry field directly in the TUI — like the current headless flow
4. If the browser can't be opened (truly headless), the user simply copies the URL from the TUI

This is essentially the headless flow + auto-browser-open, and eliminates the local HTTP form server entirely. The flow becomes:

```
User selects "Login with Claude"
  → Browser auto-opens to auth URL (best effort)
  → TUI shows: authorize URL (clickable/copyable) + code entry field
  → User authorizes in browser → copies code from callback → pastes in TUI
  → Tokens exchanged → success
```

**This should be a follow-up card** after the critical bugs in PROV-028 are fixed.

### Key Files
- `src/tui/hooks/useProviderSettingsState.ts` — lines 493-594 (separate browser/device login functions)
- `src/tui/hooks/useProviderSettingsState.ts` — lines 155-175 (two separate nav items)
- `codelet/providers/src/claude_oauth_server.rs` — entire file (local HTTP server, could be eliminated)
- `codelet/providers/src/claude_headless_login.rs` — headless flow (would become the single flow)

---

## Screenshot Reference

| Screenshot | Description |
|---|---|
| `wrong1.png` | Anthropic profile editor showing base URL `http://localhost:8888` (proxy config) |
| `wrong2.png` | Provider Settings with Anthropic expanded (login options + profile visible), Codex collapsed showing `✓ OAuth [ChatGPT]` |
| `wrong3.png` | **Claude browser OAuth flow stuck** — TUI shows "Waiting for authorization...", browser shows Anthropic callback with code, no way to paste code |
| `wrong4.png` | **Codex expanded with zero sub-items** — expanded ▼ but nothing below, status text also missing |

---

## Code Location Summary

| Component | File | Lines | Purpose |
|---|---|---|---|
| Browser OAuth server | `codelet/providers/src/claude_oauth_server.rs` | 79-168 | Main flow entry point + browser open |
| Form HTML page | `codelet/providers/src/claude_oauth_server.rs` | 384-425 | Local form for code paste (never shown) |
| TUI browser waiting render | `src/tui/components/ProviderSettingsPanel.tsx` | 295-321 | Spinner-only "Waiting for authorization..." |
| TUI headless code entry render | `src/tui/components/ProviderSettingsPanel.tsx` | 384-416 | URL + text input (headless only) |
| Nav items builder | `src/tui/hooks/useProviderSettingsState.ts` | 130-195 | Logic that produces empty Codex expansion |
| Provider row rendering | `src/tui/components/ProviderSettingsPanel.tsx` | 478-538 | Status text display |
| Browser login trigger | `src/tui/hooks/useProviderSettingsState.ts` | 493-523 | Calls NAPI binding |
| Device/headless login trigger | `src/tui/hooks/useProviderSettingsState.ts` | 528-594 | Provider-specific dispatch |
| Headless code input render | `src/tui/components/ProviderSettingsPanel.tsx` | 403-408 | No width constraint on pasted code text |
| Headless URL display | `src/tui/components/ProviderSettingsPanel.tsx` | 396-399 | Plain text, not clickable/copyable |
| OAuth input handler | `src/tui/inputHandlers/oauthModeHandler.ts` | 38-67 | Headless code entry — no copy keybind |
| Headless login (Rust) | `codelet/providers/src/claude_headless_login.rs` | all | Two-phase headless flow |
| Feature: Anthropic TUI | `spec/features/tui-anthropic-oauth-login.feature` | all | PROV-025 acceptance criteria |
| Feature: Codex TUI | `spec/features/tui-oauth-login-flow.feature` | all | PROV-017 acceptance criteria |
