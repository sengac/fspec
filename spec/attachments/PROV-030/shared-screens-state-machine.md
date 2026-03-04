# OAuth Unified Login — Shared Screens & State Machine

## Shared Screens: Success and Error

**Success (both providers):**

```
┌─ [Provider] OAuth Login ─────────────────────────────┐
│                                                       │
│  ✓ Connected to [ChatGPT / Claude]                    │
│                                                       │
│  [Enter] or [Esc] to return to provider settings      │
└───────────────────────────────────────────────────────┘
```

**Error (both providers):**

```
┌─ [Provider] OAuth Login ─────────────────────────────┐
│                                                       │
│  ✗ Login failed                                       │
│                                                       │
│  CSRF validation failed — state mismatch              │
│                                                       │
│  [Enter] Retry  [Esc] Cancel                          │
└───────────────────────────────────────────────────────┘
```

**Timeout (both providers):**

```
┌─ [Provider] OAuth Login ─────────────────────────────┐
│                                                       │
│  ✗ Login timed out                                    │
│                                                       │
│  No authorization received within 5 minutes.          │
│                                                       │
│  [Enter] Retry  [Esc] Cancel                          │
└───────────────────────────────────────────────────────┘
```

---

## Complete State Machine

### Codex (ChatGPT):

```
[Enter on "Login with ChatGPT"]
         │
         ├── server binds OK ──→ [auto-waiting]
         │                            │
         │                    ┌───────┼──────────┐
         │                    │       │          │
         │               callback   [p]      timeout/Esc
         │               arrives     │          │
         │                    │  [paste-entry]   │
         │                    │   │    │    │    │
         │                    │  Enter Esc  │    │
         │                    │   │    │  callback
         │                    │   │    │  arrives│
         │                    ▼   ▼    ▼    ▼    │
         │                 [exchanging]──────────│
         │                    │                  │
         │               ┌────┼────┐             │
         │               ▼         ▼             ▼
         │          [success]  [error]←──────[error]
         │               │         │
         │            Enter/Esc  Enter→retry / Esc→cancel
         │               │         │
         │               ▼         ▼
         │          [provider list]
         │
         └── server FAILS ──→ [paste-entry] (no back to waiting)
                                  │    │
                                Enter  Esc
                                  │    │
                            [exchanging] [provider list]
                                  │
                             ┌────┼────┐
                             ▼         ▼
                        [success]  [error]
```

### Claude (Anthropic):

```
[Enter on "Login with Claude"]
         │
         ▼
    [paste-entry]  (always — no server, no waiting mode)
         │    │
       Enter  Esc
         │    │
   [exchanging] [provider list]
         │
    ┌────┼────┐
    ▼         ▼
[success]  [error]
```

---

## PanelMode Changes

### Current modes (to remove/consolidate):
- `oauth-method-select` → **KILL** (no method selection needed)
- `oauth-browser-waiting` → rename to `oauth-auto-waiting`
- `oauth-device-waiting` → **KILL** (no device auth as separate flow)
- `oauth-headless-code-entry` → rename to `oauth-paste-entry`
- `oauth-success` → KEEP
- `oauth-error` → KEEP

### New modes:
- `oauth-auto-waiting` — spinner + keybinds, Codex only, server is listening
  - `providerId`: 'codex'
  - `authorizeUrl`: string
  - `serverPort`: number
- `oauth-paste-entry` — text input for code/URL paste
  - `providerId`: 'codex' | 'anthropic'
  - `authorizeUrl`: string
  - `codeInput`: string
  - `hasBackgroundServer`: boolean (Codex: maybe, Claude: never)
  - `warningMessage?`: string (port bind failure message)
  - `pasteHint`: string ('callback URL' vs 'code#state')
- `oauth-exchanging` — brief spinner during token exchange (NEW)
  - `providerId`: string
- `oauth-success` — KEEP as-is
- `oauth-error` — KEEP as-is

---

## Keybinding Summary

### oauth-auto-waiting (Codex only, no text input):
| Key | Action |
|-----|--------|
| `o` | Open authorize URL in browser |
| `c` | Copy authorize URL to clipboard |
| `p` | Switch to paste-entry mode |
| `Esc` | Kill server, cancel, back to provider list |

### oauth-paste-entry (both providers, text input active):
| Key | Condition | Action |
|-----|-----------|--------|
| chars | always | Append to codeInput |
| Backspace | always | Delete last char |
| `o` | input empty | Open authorize URL in browser |
| `c` | input empty | Copy authorize URL to clipboard |
| `Enter` | input not empty | Submit → exchanging |
| `Esc` | hasBackgroundServer=true | Back to auto-waiting |
| `Esc` | hasBackgroundServer=false | Cancel, back to provider list |

### oauth-exchanging:
| Key | Action |
|-----|--------|
| (none) | All input absorbed, brief spinner |

### oauth-success:
| Key | Action |
|-----|--------|
| `Enter` or `Esc` | Back to provider list |

### oauth-error:
| Key | Action |
|-----|--------|
| `Enter` | Retry (restart from beginning) |
| `Esc` | Cancel, back to provider list |

---

## What Gets Parsed From Paste Input

### Claude:
Input: `authcode123abc#verifierstate456`
Parse: split on first `#` → code = `authcode123abc`, state = `verifierstate456`
Validate: state must match pkce verifier

### Codex (when pasting callback URL):
Input: `http://localhost:1455/auth/callback?code=abc123&state=xyz789`
Parse: extract `code` and `state` from URL query params
Validate: state must match generated state

### Codex (smart parsing — accept either format):
We should try URL parsing first, then fall back to raw code+state.
This handles both:
- Full URL: `http://localhost:1455/auth/callback?code=X&state=Y`
- Just the code param if they extracted it: `abc123` (with state in separate param — probably not useful)

In practice: parse as URL first. If that fails or has no `code` param, treat as raw.

---

## NAPI Interface Changes

### Claude (simplified):
```
claude_oauth_login_start() → { authorizeUrl, pkceVerifier }    // renamed from headless_start
claude_oauth_login_complete(codeWithState, pkceVerifier) → tokens  // renamed from headless_complete
claude_oauth_browser_login() → DELETE (kill HTTP server flow)
```

### Codex (new unified entry point):
```
codex_oauth_login_start() → { authorizeUrl, state, pkceVerifier, serverPort?, serverFailed: bool }
  // Tries to bind server. Returns serverPort if OK, serverFailed=true if not.
  // Always returns authorizeUrl regardless.

codex_oauth_login_complete_from_paste(callbackUrl, expectedState, pkceVerifier) → tokens
  // Parse code+state from URL, validate, exchange

codex_oauth_login_cancel() → void
  // Kill background server if running

// KEEP existing browser_oauth_login internally — used by auto-waiting
// KEEP existing device auth internally — could be future fallback
// But neither is exposed as a separate TUI option
```

---

## Files To Change

### Kill:
- `claude_oauth_server.rs` → dead code after this
- `claude_oauth_browser_login` NAPI binding

### Rename:
- `claude_headless_login.rs` → `claude_login.rs`
- `claude_oauth_headless_start` → `claude_oauth_login_start`
- `claude_oauth_headless_complete` → `claude_oauth_login_complete`

### Modify:
- `oauthModeHandler.ts` — new keybindings for `p`, updated mode names
- `useProviderSettingsState.ts` — remove method selection, unified login start
- `ProviderSettingsPanel.tsx` — new screen renderings
- `providerSettingsModeMapper.ts` — updated mode mappings
- `codex_oauth.rs` NAPI — new unified start/complete functions

### Remove from TUI:
- `oauth-method-select` mode and all references
- `oauth-device-waiting` mode and all references
- Separate "browser" / "headless" nav items in provider expansion
