# PROV-029 — Final Design Decisions

## Status

SPECIFYING complete. All questions answered, rules/examples/feature file rewritten.

---

## Design Decisions (Q&A Summary)

### Profiles: OpenAI API only
Profiles exist ONLY for the "OpenAI API" provider (local models: vLLM, Ollama, etc.).
No other provider shows profiles or "Create new profile". `saveProfile()` rejects
any providerId that is not `openai`. The Rust session layer uses
`provider:profile/model-id` format — the guard is on the TUI display side,
not the data layer.

### Keybinds: Enter and 'd' only
- **Enter** — acts on the selected item (expand, edit, start login, create)
- **d** — destructive action with confirmation (delete key, disconnect OAuth, delete profile)
- **No `e`** — removed entirely. Enter on 🔑 API key opens editor.
- **No `n`** — removed entirely. Enter on `+ Create new profile` creates.
- **No `t`** — removed entirely. Connection testing belongs in the models view.

### Delete confirmations: uniform for all destructive actions
Extend the existing `delete-profile` y/n dialog pattern to also cover
`delete-api-key` and `disconnect-oauth` modes.

### Dead code deletion: in this card
ProviderSettingsView.tsx (880 lines), useProviderProfiles.ts (414 lines),
and dead types in provider.ts. Mechanical deletion, not a separate feature.

### Provider list: 16 providers (5 removed)
Removed for no tool calling support:
- **Perplexity** — tools explicitly warned as "not supported" (perplexity.rs:362-368)
- **Hyperbolic** — tools explicitly warned as "not supported" (hyperbolic.rs:264-269)
- **Mira** — tools logged as "will be ignored" (mira.rs:346-349)
- **Voyage AI** — embedding-only provider, Completion = Nothing (voyageai.rs:42-51)

Removed as redundant:
- **Ollama** — uses OpenAI-compatible API, configure as profiles under OpenAI API

### OpenAI renamed to "OpenAI API"
It's a local-model API-compatible format (vLLM, Ollama), NOT the OpenAI cloud
service (which is Codex/ChatGPT via OAuth). Profile-only — no API key row,
no env var credentials. `+ Create new profile` always visible.

### 🔑 API key row visibility
Show for providers with `requiresApiKey: true` or `envVar` defined. Not for
profile-only providers (OpenAI API).

### Headless OAuth
Both browser and headless login nav items are included. Inconsistency between
Codex and Claude headless flows is tracked as PROV-030.

### Context-sensitive footer
Footer keybind hints change based on selected item type. Always includes
`/ filter · Tab: Switch to models · Esc: close`. Item-specific hints are
prepended (e.g. `Enter: expand` for provider rows, `d: disconnect` for OAuth status).

### Tab hint
"Tab: Switch to models" on provider settings panel. "Tab: Switch to providers"
on models panel.

### PROVIDER_ENV_VARS
Add `codex: ['CODEX_API_KEY']` to credentials.ts. Rust-side fix out of scope.

---

## Final Provider List (16 total)

**OAuth:** Anthropic, Codex (ChatGPT)

**Cloud API-key:** Cohere, Google Gemini, Mistral AI, xAI, Together AI,
Hugging Face, OpenRouter, Groq, DeepSeek, Moonshot, Galadriel, Azure OpenAI, Z.AI

**Profile-only (local models):** OpenAI API

---

## Scope Boundary

**In this card:**
- Profile guards: only OpenAI API gets profiles (buildNavItems, reload, saveProfile, header)
- New `api-key` nav item type in buildNavItems
- Remove `e`, `n`, `t` keybinds from listModeHandler
- Delete confirmations for API key and OAuth disconnect
- oauth-status 'd' handler (disconnect with confirmation)
- Context-sensitive footer per item type
- Tab hint: "Switch to models" / "Switch to providers"
- PROVIDER_ENV_VARS codex entry (TS side)
- Dead code deletion (ProviderSettingsView.tsx, useProviderProfiles.ts, dead types)
- Remove providers: ollama, perplexity, hyperbolic, mira, voyageai (TS side)
- Rename openai → "OpenAI API", make profile-only

**NOT in this card (follow-up):**
- ProviderDisplayStatus.source type mismatch (3 definitions)
- edit-api-key blank field UX
- DEFAULT_PROFILE_BASE_URL = localhost:8888 confusion
- Header "(25 items)" mixing providers with sub-items
- Rust-side provider removals (resolver.rs)
- Rust-side codex entry (resolver.rs)
- Headless vs browser OAuth inconsistency (PROV-030)
