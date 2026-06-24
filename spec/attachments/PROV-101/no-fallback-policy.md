# PROV-101 — Remove ALL Provider/Model/Profile Selection Fallbacks

**Status:** Investigation complete (DeepSearch). This is the FIRST card and the root-cause class
behind the "Enter on OpenAI profile shows Anthropic" family of bugs.

## Policy: NO SILENT SELECTION FALLBACKS

When a provider, model, or profile cannot be **explicitly** resolved from real state
(credentials, user selection, persisted config), the code MUST NOT silently pick something.
It must instead:

- return an explicit error (`Result::Err`) **or**
- surface an explicit empty / "not configured" state to the UI,

never substitute a hardcoded default (especially not anthropic/claude) and never quietly
re-index into an unrelated list.

A "fallback" for the purposes of this card = any code path that *chooses* a provider/model/profile
when none was explicitly resolvable: `unwrap_or`/`unwrap_or_else`/`unwrap_or_default` that yields a
provider/model id, `.or(...)` provider defaults, priority chains, catch-all `_ =>` arms that pick a
provider/model, or "first selectable / index 0" cursor defaults.

## Inventory of selection fallbacks to REMOVE (from DeepSearch)

| # | File | Line(s) | Code / behaviour |
|---|------|---------|------------------|
| 1 | `codelet/sessions/src/handle_impl.rs` | ~86-88 | `create_session()`: `.get_default_model().unwrap_or_else(\|\| "anthropic/claude-opus-4-5".to_string())` |
| 2 | `codelet/sessions/src/handle_impl.rs` | ~816-818 | `create_isolated_session()`: same default-to-anthropic |
| 3 | `codelet/providers/src/manager.rs` | ~718-745 | `detect_default_provider()` priority chain (Claude > Gemini > ZAI > Codex > Copilot > OpenAI). Call sites ~287, ~405 |
| 4 | `codelet/fspec-tui/src/views/model_selector/rows.rs` | ~137-139 | `first_selectable_or_zero()` → `.position(\|r\| r.selectable).unwrap_or(0)` |
| 5 | `codelet/fspec-tui/src/views/model_selector/mod.rs` | ~133-139 (also 164,278,449,460,467,534,539) | cursor seeding else-branch falls back to `first_selectable_or_zero` |
| 6 | `codelet/providers/src/models/fallback_models.json` | 2-60 | hardcoded anthropic/claude catalog used as selection seed |
| 7 | `codelet/providers/src/models/registry.rs` | ~284-347 | hardcoded in-code default registry inserting anthropic + claude models |
| 8 | `codelet/providers/src/models/cache.rs` | ~186-208 | hardcoded anthropic/claude cache seed |

### Priority guidance

- **#1–#5 are MANDATORY removals** — they are the true "pick something silently" selection fallbacks.
- **#3** must be replaced by: when no provider is explicitly selected, return
  `ProviderError` ("no provider explicitly selected") rather than auto-picking Claude.
- **#1/#2** must be replaced by: require an explicit model; if `get_default_model()` is `None`,
  return an error (no anthropic substitution).
- **#4/#5** the model-selector cursor must NOT default to index 0 when the current model is absent;
  surface "no current model / nothing selected" rather than silently highlighting the first row.
- **#6–#8 (catalog/registry/cache):** these are the bundled offline catalog. Remove their use as a
  *selection default*. If they must remain as an offline catalog, they must not be consulted to
  *auto-choose* a provider/model — only to look up metadata for an already-explicitly-chosen id.
  Decide per Example Mapping whether to delete or merely de-privilege (no anthropic-first ordering).

## NOT fallbacks (do NOT touch — confirmed by DeepSearch)

These matched search patterns but do not *select* a provider/model:

- `provider_settings/projection.rs` `pretty_name` / `oauth_login_methods` catch-all `_ =>` → returns
  the provider's own id/display name (display label, not a choice).
- `credentials/resolver.rs` `get_provider_env_vars`, `split('/').next().unwrap_or("")`, and the
  `claude_auth.json` credential fallback (only for an already-chosen anthropic provider).
- `handle_impl.rs:1180` `"anthropic" | "codex" | "github-copilot" => "oauth"` auth-type classifier.
- Boolean/string `unwrap_or(false)` / `unwrap_or_default()` / `unwrap_or("")` field defaults.
- `model_limits.rs`, `claude.rs`, `types.rs` hardcoded `claude-*` strings = test fixtures / doc comments.

## Acceptance direction

- No selection path silently yields anthropic/claude when input is missing.
- Missing/unresolvable model or provider → explicit error or explicit empty UI state.
- Model-selector cursor does not auto-snap to index 0 to "select" a model when current is absent.
- Tests must run fully offline (path-injectable config, dummy creds, no network).
