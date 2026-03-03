# Provider Settings TUI — Proposed Mockups

## Current state (broken)

```
Provider Settings (25 items)

> ► OpenAI
  ▼ Anthropic ✓ OAuth [Claude] (1 profile)    ← WRONG: "(1 profile)" shown
        ✓ OAuth [Claude]
        🔑 Login with Claude (browser)
        🔑 Login with Claude (headless)
        📁 anthropic → http://localhost:8888    ← WRONG: stale profile shown
  ► Cohere (not configured)
  ► Google Gemini ✓ AIza•••••••H3Ck [env]
  ► Ollama (not configured)
  ...
  ► Codex (ChatGPT) ✓ OAuth [ChatGPT]
```

Problems visible:
- Anthropic shows "(1 profile)" — profile count for OAuth provider
- Stale profile row `📁 anthropic → http://localhost:8888` visible
- Every cloud provider would show "Create Profile" if expanded (not shown because collapsed)

---

## Proposed: Option A — Profiles only under providers that HAVE profiles

The key insight: profiles are already stored per-provider in config. The TUI
should only show profile UI for providers that actually have profiles configured.
Cloud providers won't have any (and we block creating new ones for cloud),
while local-compatible providers like OpenAI (for vLLM) will show them naturally.

### Anthropic expanded (OAuth, has ANTHROPIC_API_KEY too)

```
Provider Settings (25 items)

  ► OpenAI ✓ sk-proj••••••q8 [env]
  ▼ Anthropic ✓ OAuth [Claude]
        ✓ OAuth [Claude]                       ← OAuth status (d=disconnect, t=test)
        🔑 Login with Claude (browser)         ← Enter to start OAuth
        🔑 Login with Claude (headless)        ← Enter to start headless
  ► Cohere (not configured)
> ► Google Gemini ✓ AIza•••••••H3Ck [env]
  ...
```

- No profile rows (OAuth guard filters them in display)
- No "(N profiles)" count
- No "Create Profile" button
- `e` key → opens API key editor (yes, Anthropic CAN use an API key)
- `d` key → disconnect confirmation (when OAuth tokens exist)

### Anthropic expanded (API key only, no OAuth tokens)

```
  ▼ Anthropic ✓ sk-ant-••••••Qr7K [env]
        🔑 Login with Claude (browser)         ← OAuth login options still shown
        🔑 Login with Claude (headless)
```

- No OAuth status row (no tokens)
- Login options still visible (user can choose to OAuth instead)
- `e` key → opens API key editor
- `d` key → delete API key confirmation

### Codex expanded (OAuth only, requiresApiKey: false)

```
  ▼ Codex (ChatGPT) ✓ OAuth [ChatGPT]
        ✓ OAuth [ChatGPT]                      ← OAuth status
        🔑 Login with ChatGPT (browser)
        🔑 Login with ChatGPT (headless)
```

- Same as Anthropic: no profiles, no profile count
- `e` key → opens API key editor (user might want to set CODEX_API_KEY)
- `d` key → disconnect OAuth confirmation

### OpenAI expanded (API key, has local profiles)

```
  ▼ OpenAI ✓ sk-proj••••••q8 [env] (2 profiles)
        📁 work-vllm → http://10.0.1.5:8080   ← local vLLM server
        📁 home-ollama → http://localhost:11434 ← local Ollama
        + Create new profile
```

- Profiles shown (these are real local server configs)
- "(2 profiles)" count in header
- "Create Profile" button available
- `e` key → API key editor
- `n` key → create new profile
- `d` key on profile → delete profile confirmation
- `d` key on provider row → delete API key confirmation

### Google Gemini expanded (API key, no profiles)

```
  ▼ Google Gemini ✓ AIza•••••••H3Ck [env]
```

- No sub-items at all (no profiles, no OAuth)
- Clean empty expansion
- `e` key → API key editor
- `d` key → delete API key confirmation (WITH confirmation dialog)

### Ollama expanded (no auth, has profiles)

```
  ▼ Ollama
        📁 gpu-server → http://10.0.1.20:11434
        + Create new profile
```

- No API key shown (authMethod: 'none')
- Profiles shown (local server configs)
- `n` key → create new profile

---

## Proposed: Option B — Guard profiles by isOAuthProvider only

Same as Option A for OAuth providers (Anthropic, Codex), but cloud API-key
providers like OpenAI/Gemini would ALSO show "Create Profile" when expanded:

### OpenAI expanded (API key, no profiles configured)

```
  ▼ OpenAI ✓ sk-proj••••••q8 [env]
        + Create new profile                   ← always visible for non-OAuth
```

This feels wrong — why would you create a "profile" for cloud OpenAI?

### Google Gemini expanded (API key, no profiles)

```
  ▼ Google Gemini ✓ AIza•••••••H3Ck [env]
        + Create new profile                   ← confusing for cloud provider
```

---

## Recommendation: Option A

Option A is cleaner because:
1. "Create Profile" only appears where it makes sense (providers that already have profiles OR Ollama which is inherently local)
2. No confusing "Create Profile" on Google Gemini or Mistral
3. The guard is simple: don't show profile UI for providers with 0 profiles UNLESS they're a known local provider
4. OAuth providers are naturally handled (they'll never have profiles because saveProfile rejects them)

However — there's a chicken-and-egg: if a provider has 0 profiles, how does a user create their first one? They'd only see "Create Profile" on Ollama.

**Better refinement**: Show "Create Profile" for ALL non-OAuth providers. But DON'T show profile rows or counts for OAuth providers. This way:
- OpenAI shows "Create Profile" when expanded (user can add a vLLM profile)
- Gemini shows "Create Profile" when expanded (user can add a local Gemini-compatible server)
- Anthropic does NOT show "Create Profile" (OAuth — no profiles)
- Codex does NOT show "Create Profile" (OAuth — no profiles)

This is actually what the CURRENT code does (Create Profile button IS guarded by isOAuthProvider) — the only bugs are:
1. Stale profile ROWS still show for OAuth providers
2. Profile COUNT shows in header for OAuth providers
3. Keybind bugs (n/d/e/t)
