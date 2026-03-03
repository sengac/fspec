# Provider Settings TUI — Final Mockups (v3)

## Design principle

Every way to authenticate is a visible, selectable item when expanded.
No hidden keybind behavior. Only Enter and 'd'. Profiles only for OpenAI API.

---

## Anthropic (OAuth connected + API key from env)

```
  ▼ Anthropic ✓ OAuth [Claude]
        ✓ OAuth [Claude]                       ← d: disconnect (confirm)
        🔑 Login with Claude (browser)         ← Enter: start OAuth
        🔑 Login with Claude (headless)        ← Enter: start headless
        🔑 API key ✓ sk-ant-••••Qr7K [env]    ← Enter: edit, d: delete (confirm)
```

## Anthropic (OAuth connected, no API key)

```
  ▼ Anthropic ✓ OAuth [Claude]
        ✓ OAuth [Claude]
        🔑 Login with Claude (browser)
        🔑 Login with Claude (headless)
        🔑 API key (not set)                   ← Enter: set key
```

## Anthropic (API key only, no OAuth)

```
  ▼ Anthropic ✓ sk-ant-••••Qr7K [env]
        🔑 Login with Claude (browser)
        🔑 Login with Claude (headless)
        🔑 API key ✓ sk-ant-••••Qr7K [env]    ← Enter: edit, d: delete (confirm)
```

## Codex (OAuth connected)

```
  ▼ Codex (ChatGPT) ✓ OAuth [ChatGPT]
        ✓ OAuth [ChatGPT]
        🔑 Login with ChatGPT (browser)
        🔑 Login with ChatGPT (headless)
        🔑 API key (not set)
```

## Google Gemini (API key from env)

```
  ▼ Google Gemini ✓ AIza•••••••H3Ck [env]
        🔑 API key ✓ AIza•••••••H3Ck [env]    ← Enter: edit, d: delete (confirm)
```

## Mistral AI (not configured)

```
  ▼ Mistral AI (not configured)
        🔑 API key (not set)                   ← Enter: set key
```

## OpenAI API (2 profiles)

```
  ▼ OpenAI API (2 profiles)
        📁 work-vllm → http://10.0.1.5:8080   ← Enter: edit, d: delete (confirm)
        📁 home-ollama → http://localhost:11434
        + Create new profile                   ← Enter: create
```

No 🔑 API key row. Profile-only provider.

## OpenAI API (no profiles yet)

```
  ▼ OpenAI API
        + Create new profile                   ← Enter: create
```

---

## Full screen mockup

```
Provider Settings

> ► OpenAI API (2 profiles)
  ▼ Anthropic ✓ OAuth [Claude]
        ✓ OAuth [Claude]
        🔑 Login with Claude (browser)
        🔑 Login with Claude (headless)
        🔑 API key ✓ sk-ant-••••Qr7K [env]
  ► Cohere (not configured)
  ► Google Gemini ✓ AIza•••••••H3Ck [env]
  ► Mistral AI (not configured)
  ► xAI (not configured)
  ► Together AI (not configured)
  ► Hugging Face (not configured)
  ► OpenRouter (not configured)
  ► Groq (not configured)
  ► DeepSeek (not configured)
  ► Moonshot (not configured)
  ► Galadriel (not configured)
  ► Azure OpenAI (not configured)
  ► Z.AI ✓ 5fc6d5•••••••NHC7 [env]
  ► Codex (ChatGPT) ✓ OAuth [ChatGPT]

Enter: expand · / filter · Tab: Switch to models · Esc: close
```

16 providers. No Ollama, Perplexity, Hyperbolic, Mira, or Voyage AI.

---

## Keybind behavior (item-type specific)

Arrow keys to navigate, Enter to act, d to delete. That's it.

| Cursor on...          | Enter              | d                    |
|-----------------------|--------------------|----------------------|
| Provider row          | toggle expand      | —                    |
| ✓ OAuth [...]         | —                  | disconnect (confirm) |
| 🔑 Login (browser)   | start OAuth        | —                    |
| 🔑 Login (headless)  | start headless     | —                    |
| 🔑 API key           | open key editor    | delete key (confirm) |
| 📁 profile           | edit profile       | delete (confirm)     |
| + Create new profile  | create profile     | —                    |

No 'e'. No 'n'. No 't'.

---

## Context-sensitive footer

| Cursor on...          | Footer                                                                    |
|-----------------------|---------------------------------------------------------------------------|
| Provider row          | Enter: expand · / filter · Tab: Switch to models · Esc: close             |
| ✓ OAuth [...]         | d: disconnect · / filter · Tab: Switch to models · Esc: close             |
| 🔑 Login (browser)   | Enter: start login · / filter · Tab: Switch to models · Esc: close        |
| 🔑 Login (headless)  | Enter: start login · / filter · Tab: Switch to models · Esc: close        |
| 🔑 API key           | Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close   |
| 📁 profile           | Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close   |
| + Create new profile  | Enter: create · / filter · Tab: Switch to models · Esc: close             |
