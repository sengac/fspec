# Codex Model Catalog Research

**Source:** https://github.com/openai/codex (cloned and analyzed 2026-03-04)
**Key file:** `codex-rs/core/models.json` (bundled fallback catalog)

## Architecture Overview

The Codex CLI uses a dynamic model system:

1. **Bundled catalog** (`models.json`) — 12 models shipped with the binary as fallback
2. **Dynamic `/models` endpoint** — fetched on startup, cached 300s (server-side plan filtering)
3. **`ModelPreset::filter_by_auth()`** — client-side filter: ChatGPT OAuth mode shows all; API key mode filters by `supported_in_api`
4. **`ModelVisibility`** — `list` (shown in picker) vs `hide` (usable but not shown) vs `none`

## Bundled Models (models.json)

| Slug | Visibility | Priority | Plans |
|------|-----------|----------|-------|
| `gpt-5.3-codex` | **list** | 0 | All except `free` |
| `gpt-5.2-codex` | **list** | 3 | All including `free` |
| `gpt-5.1-codex-max` | **list** | 4 | All including `free` |
| `gpt-5.1-codex` | hide | 5 | All including `free` |
| `gpt-5.2` | **list** | 6 | All including `free` |
| `gpt-5.1` | hide | 7 | All including `free` |
| `gpt-5-codex` | hide | 10 | All including `free` |
| `gpt-5` | hide | 11 | All including `free` |
| `gpt-oss-120b` | hide | 11 | All including `free` |
| `gpt-oss-20b` | hide | 11 | All including `free` |
| `gpt-5.1-codex-mini` | **list** | 12 | All including `free` |
| `gpt-5-codex-mini` | hide | 13 | All including `free` |

### Key observations:
- Only **5 models** visible in picker (`list`): gpt-5.3-codex, gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.2, gpt-5.1-codex-mini
- All 12 models have `supported_in_api: true`
- All 12 have `context_window: 272000` except gpt-oss-120b and gpt-oss-20b (128000)

## Models NOT Available in Codex

These models from models.dev / OpenAI platform are NOT in the Codex catalog:

### Reasoning models (none available):
- o3-pro, o3, o4-mini, o1-pro, o1, o3-mini

### GPT-4 family (none available):
- gpt-4.1, gpt-4.1-mini, gpt-4.1-nano, gpt-4o, gpt-4o-2024-11-20

### GPT-5 variants not in Codex:
- gpt-5-pro, gpt-5-mini, gpt-5-nano
- gpt-5.2-pro, gpt-5.2-chat-latest
- gpt-5.1-mini, gpt-5.1-nano

## Server-Side Filtering

The `available_in_plans` field exists in `models.json` but is **NOT consumed by client code**:
- No Rust source references `available_in_plans`
- Filtering happens server-side on the `/models` endpoint
- Server returns only models the user's subscription supports
- Client version is sent as query param for version-gating

## Subscription Plans

Plans recognized by Codex: `free`, `go`, `plus`, `pro`, `team`, `business`, `enterprise`, `edu`, `education`, `finserv`, `hc`

## Client Auth Mode Flow

```
refresh_available_models():
  if auth_mode != ChatGPT:
    → skip network fetch, use bundled/cached only
  if ChatGPT auth:
    → fetch from /models endpoint (server filters by plan)
    → merge with bundled catalog
    → cache result for 300s
```

## Filtering Implementation in Codex

```rust
// ModelPreset::filter_by_auth
pub fn filter_by_auth(models: Vec<ModelPreset>, chatgpt_mode: bool) -> Vec<ModelPreset> {
    models.into_iter()
        .filter(|model| chatgpt_mode || model.supported_in_api)
        .collect()
}
```

In ChatGPT mode, ALL models pass through (no client-side filtering).
The server endpoint is the gatekeeper.

## Implications for Our Implementation

1. We cannot call the Codex `/models` endpoint ourselves (requires Codex OAuth tokens + specific base URL)
2. We should maintain a local allowlist derived from the bundled `models.json` slugs
3. The allowlist should be updatable (not hardcoded) — ideally fetched or configurable
4. Models from models.dev that don't match the Codex allowlist should be filtered out when Codex OAuth is active
5. Consider using slug prefix matching (how Codex does it) rather than exact matching — e.g. `gpt-5.2-codex` matches `gpt-5.2-codex-something`
