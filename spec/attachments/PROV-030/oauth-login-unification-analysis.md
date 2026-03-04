# OAuth Login Unification Analysis — PROV-030

## Problem Statement

The current OAuth login architecture offers **four separate login paths** across two providers, each with a "browser" and "headless" variant. This creates UX confusion ("which one do I pick?") and unnecessary code duplication. Neither approach is ideal on its own — the best UX is a unified flow that combines the strengths of both.

---

## Current State: Four Login Paths

### Anthropic (Claude) — Two Flows

| Flow | Module | Mechanism |
|------|--------|-----------|
| **Browser login** | `claude_oauth_server.rs` | Starts local HTTP server on ephemeral port → opens browser to **local form page** → user authorizes on claude.ai → copies `code#state` from Anthropic callback → pastes into local form → state validated → tokens exchanged |
| **Headless login** | `claude_headless_login.rs` | Generates PKCE → invokes code-entry callback with authorize URL → user visits URL on another device → copies `code#state` → pastes via callback → state validated → tokens exchanged |

### Codex (ChatGPT) — Two Flows

| Flow | Module | Mechanism |
|------|--------|-----------|
| **Browser login** | `codex_oauth_server.rs` | Starts local HTTP server on port 1455 → opens browser to OpenAI authorize URL → OpenAI redirects back to `localhost:1455/auth/callback` with code+state in query params → **automatic** → tokens exchanged |
| **Device auth** | `codex_device_auth.rs` | POSTs to OpenAI device endpoint → gets user_code + verification URL → displays to user → polls token endpoint → user authorizes on another device → polling detects completion → tokens exchanged |

---

## Analysis: Why Neither Approach Is Good

### Codex browser flow works better because:
- **It's truly automatic.** OpenAI redirects back to `localhost:1455/auth/callback` — the user just authorizes in the browser and it's done. No copy-pasting.

### Anthropic browser flow is awkward because:
- Anthropic's `redirect_uri` is **remote** (`console.anthropic.com/oauth/code/callback`), NOT localhost. So the local server can't receive a direct redirect callback.
- Instead, it opens a **local form page** where the user must manually copy a `code#state` string from the Anthropic callback page and paste it in. This is essentially the **same UX as headless** — just wrapped in a local HTTP page instead of a terminal prompt.

### Key Insight: Anthropic "browser" ≈ Anthropic "headless"
The only difference is:
- **Browser**: opens a local web page with a form
- **Headless**: shows URL in terminal, pastes in terminal

Both require the user to manually copy-paste `code#state`. The "browser" version just adds an unnecessary HTTP server to do something a terminal can do natively.

### Codex device auth is overkill for most cases
- The RFC 8628 device auth flow (show a code, go to a URL, enter the code) is designed for devices without text input — TVs, IoT. In a terminal, it's an unnecessary indirection when you could just open the browser.

---

## Proposed Architecture: One Login Per Provider

### Unified Login Flow

```
┌─────────────────────────────────────────────┐
│            Unified Login Flow               │
├─────────────────────────────────────────────┤
│ 1. Generate PKCE + build authorize URL      │
│ 2. Attempt: open browser to authorize URL   │
│ 3. Display: show URL in terminal (always)   │
│    - "Press 'o' to open in browser"         │
│    - "Or copy this URL: https://..."        │
│ 4. Wait for auth to complete:               │
│    ┌──────────────┬────────────────────┐    │
│    │ Codex        │ Anthropic          │    │
│    │ Start local  │ Show text input    │    │
│    │ server, wait │ for code#state     │    │
│    │ for redirect │ paste              │    │
│    └──────────────┴────────────────────┘    │
│ 5. Exchange code → persist tokens           │
└─────────────────────────────────────────────┘
```

### What changes concretely:

#### For Anthropic:
- **Kill `claude_oauth_server.rs`** (the browser flow with the form page). It's unnecessary overhead — spinning up a hyper HTTP server just to show a paste form that the terminal can do natively.
- **Keep `claude_headless_login.rs`** as the sole flow, but rename it to `claude_login.rs`.
- In the TUI: one "Login with Claude" option → shows authorize URL, offers to open browser (just `open::that(url)`), shows text input for `code#state`. This is what the headless code entry mode already does.
- The `'o'` to open in browser and `'c'` to copy URL shortcuts in the headless code entry handler **already exist** (PROV-028) — they just need to be the default/only UX.

#### For Codex:
- **Merge browser + device auth into one flow.** Try to start the local HTTP server and open the browser. If that works, the redirect callback handles it automatically. If the port is busy or the browser can't open, **fall back** to showing the URL + a device-code-style flow.
- In the TUI: one "Login with ChatGPT" option → attempts browser redirect first, shows URL in terminal as fallback, degrades to device auth if needed.

### TUI Impact

**Before (4 options across 2 providers):**
```
Login with Claude (browser)     ← kill this
Login with Claude (headless)    ← rename to "Login with Claude"

Login with ChatGPT (browser)    ← merge into "Login with ChatGPT"
Login with ChatGPT (headless)   ← auto-fallback
```

**After (2 options, one per provider):**
```
Login with Claude               ← show URL, open browser, text input for code#state
Login with ChatGPT              ← try auto-redirect, fallback to device auth
```

### NAPI Simplification

#### Claude:
- **Keep**: `claude_oauth_headless_start()` + `claude_oauth_headless_complete()` → rename to `claude_oauth_login_start()` + `claude_oauth_login_complete()`
- **Kill**: `claude_oauth_browser_login()` (the HTTP server version)

#### Codex:
- **Keep**: `codex_oauth_browser_login()` as the primary (auto-redirect)
- **Keep**: `codex_oauth_device_login_start()` + `codex_oauth_device_login_poll()` as internal fallback
- **New**: `codex_oauth_login()` that tries browser first, falls back to device auth

---

## Summary Table

| What | Current | Proposed |
|------|---------|----------|
| Claude TUI options | 2 (browser, headless) | 1 ("Login with Claude") |
| Claude Rust modules | 3 (`claude_oauth.rs`, `claude_oauth_server.rs`, `claude_headless_login.rs`) | 2 (`claude_oauth.rs`, `claude_login.rs`) |
| Codex TUI options | 2 (browser, headless) | 1 ("Login with ChatGPT") |
| Codex Rust modules | 3 (`codex_oauth.rs`, `codex_oauth_server.rs`, `codex_device_auth.rs`) | Same 3, but unified entry point |
| User mental model | "Which login type do I pick?" | "Login" → it figures it out |

---

## Files Involved

### Rust (codelet/providers/src/)
- `claude_oauth.rs` — Core PKCE, URL building, token exchange, headers (KEEP)
- `claude_oauth_server.rs` — Browser flow HTTP form server (KILL — replaced by unified flow)
- `claude_headless_login.rs` — Headless code-paste flow (KEEP — rename to `claude_login.rs`)
- `claude_auth.rs` — Token persistence (KEEP)
- `codex/codex_oauth.rs` — Core PKCE, JWT, URL building, token exchange (KEEP)
- `codex/codex_oauth_server.rs` — Browser redirect callback server (KEEP — used as primary)
- `codex/codex_device_auth.rs` — RFC 8628 device auth (KEEP — used as fallback)
- `codex/codex_auth.rs` — Token persistence (KEEP)

### NAPI (codelet/napi/src/)
- `claude_oauth.rs` — Bindings: `claude_oauth_browser_login`, `claude_oauth_headless_start`, `claude_oauth_headless_complete`, etc.
- `codex_oauth.rs` — Bindings: `codex_oauth_browser_login`, `codex_oauth_device_login_start`, `codex_oauth_device_login_poll`, etc.

### TUI (src/tui/)
- `inputHandlers/oauthModeHandler.ts` — Keyboard handling for OAuth modes
- `hooks/useProviderSettingsState.ts` — OAuth state management
- `components/ProviderSettingsPanel.tsx` — OAuth UI rendering
- `utils/providerSettingsModeMapper.ts` — Mode mappings
- `utils/providerSettingsHelpers.ts` — Provider helpers

### Feature Specs (spec/features/)
- `claude-oauth-browser-login.feature` — Browser flow spec (DEPRECATE)
- `claude-headless-login.feature` — Headless flow spec (UPDATE)
- `codex-oauth-login.feature` — Combined spec (UPDATE)
- `device-auth-flow.feature` — Device auth spec (UPDATE)
- `tui-oauth-login-flow.feature` — TUI login spec (UPDATE)
- `tui-anthropic-oauth-login.feature` — Anthropic TUI spec (UPDATE)
- `browser-oauth-callback-server.feature` — Server spec (DEPRECATE for Anthropic)

---

## TUI Mockup Areas to Design

1. **Unified Claude login screen** — URL display + open browser shortcut + code#state input
2. **Unified Codex login screen** — Auto-redirect with fallback to device code display
3. **Error/retry UX** — Same for both providers
4. **Provider settings list** — One "Login with ..." option per OAuth provider
5. **Success/disconnect flow** — Unchanged
