# OAuth Unified Login — TUI Mockups & User Flow Design

## Starting Point: Provider Settings List

```
  ▾ anthropic          (not configured)
    ├─ Login with Claude
    └─ Edit API key

  ▾ codex              (not configured)
    ├─ Login with ChatGPT
    └─ Edit API key
```

User presses Enter on "Login with ChatGPT" or "Login with Claude".

---

## CODEX: Full User Journey

### What happens behind the scenes on Enter:

1. Generate PKCE + random state
2. Try to bind to port 1455
3. Build authorize URL (redirect_uri = `localhost:1455/auth/callback`)
4. Try `open::that(authorize_url)`

Two paths diverge based on step 2.

---

### PATH A: Server started successfully

**Screen: Auto-Waiting**

```
┌─ ChatGPT OAuth Login ────────────────────────────────┐
│                                                       │
│  ⠋ Waiting for authorization...                       │
│                                                       │
│  Browser should have opened automatically.            │
│  Listening for callback on localhost:1455              │
│                                                       │
│  https://auth.openai.com/oauth/authorize?client_...   │
│                                                       │
│  [o] Open URL  [c] Copy URL  [p] Paste code           │
│  [Esc] Cancel                                         │
└───────────────────────────────────────────────────────┘
```

NOT a text input mode. Waiting screen with shortcut keys. No cursor, no typing.

**Events:**

| Event | Result |
|-------|--------|
| OAuth redirect callback arrives on :1455 | → Exchange tokens → Success screen |
| User presses `o` | Opens browser to authorize URL |
| User presses `c` | Copies URL to clipboard |
| User presses `p` | → Paste screen (server stays alive in background) |
| User presses `Esc` | Kill server, cancel, back to provider list |
| 5 minute timeout | → Error screen ("timed out") |

Happy path: user authorizes in browser → OpenAI redirects → callback hits server → auto-completes.

If redirect didn't work (proxy, firewall, browser oddity), user presses `p`:

**Screen: Paste Entry (with background server)**

```
┌─ ChatGPT OAuth Login ────────────────────────────────┐
│                                                       │
│  Paste the callback URL from your browser's           │
│  address bar:                                         │
│                                                       │
│  > ▌                                                  │
│                                                       │
│  [o] Open URL  [c] Copy URL  (when input is empty)    │
│  [Enter] Submit  [Esc] Back                           │
└───────────────────────────────────────────────────────┘
```

Text input mode. Cursor active.

**Keybindings:**
- Characters → append to input
- Backspace → delete last char
- `o` when input empty → open browser
- `c` when input empty → copy URL
- `Enter` with text → parse code+state from URL → exchange
- `Esc` → back to auto-waiting (server still running!)

User pastes: `http://localhost:1455/auth/callback?code=abc123&state=xyz789`

We parse `code` and `state` from query params. Exchange tokens. Success.

**CRITICAL:** While in paste mode, server is still listening. If redirect arrives while typing, auto-completes and jumps to success. Whichever happens first wins.

---

### PATH B: Server failed to start

Port 1455 busy or bind error. No redirect possible. Still build authorize URL with redirect_uri = `localhost:1455/auth/callback` and try to open browser.

**Screen: Paste Entry (no server, straight to input)**

```
┌─ ChatGPT OAuth Login ────────────────────────────────┐
│                                                       │
│  ⚠ Could not start callback server on port 1455       │
│                                                       │
│  After authorizing, your browser will show a          │
│  connection error. Copy the FULL URL from your        │
│  browser's address bar and paste it below.            │
│                                                       │
│  https://auth.openai.com/oauth/authorize?client_...   │
│                                                       │
│  > ▌                                                  │
│                                                       │
│  [o] Open URL  [c] Copy URL  (when input is empty)    │
│  [Enter] Submit  [Esc] Cancel                         │
└───────────────────────────────────────────────────────┘
```

No waiting screen. No `[p]` needed. Straight to text input.

`Esc` = cancel entirely (back to provider list) — no waiting mode to return to.

User flow:
1. Opens browser (via `o` or it already opened)
2. Authorizes on OpenAI
3. Browser redirects to `http://localhost:1455/auth/callback?code=abc&state=xyz`
4. Browser shows "This site can't be reached" — URL bar has the code
5. User copies URL, pastes into input
6. Enter → parse code+state → exchange → success

---

## CLAUDE: Full User Journey

### What happens behind the scenes on Enter:

1. Generate PKCE (state = verifier, Anthropic convention)
2. Build authorize URL
3. Try `open::that(authorize_url)`

No server. Anthropic redirect_uri is remote (`console.anthropic.com/oauth/code/callback`). Never an auto-complete path.

**Screen: Paste Entry (always)**

```
┌─ Claude OAuth Login ─────────────────────────────────┐
│                                                       │
│  Authorize URL:                                       │
│  https://claude.ai/oauth/authorize?client_id=...      │
│                                                       │
│  After authorizing, paste the code#state below:       │
│                                                       │
│  > ▌                                                  │
│                                                       │
│  [o] Open in browser  [c] Copy URL                    │
│  [Enter] Submit  [Esc] Cancel                         │
└───────────────────────────────────────────────────────┘
```

Same pattern as Codex paste entry, but:
- No warning (this is normal flow, not fallback)
- Hint says "code#state" instead of "callback URL"
- `Esc` always cancels (no waiting mode)

User flow:
1. Browser opens (or press `o`)
2. Authorizes on claude.ai
3. Anthropic callback page shows `code#state`
4. Copies, pastes into input
5. Enter → split on `#` → validate state = verifier → exchange → success
