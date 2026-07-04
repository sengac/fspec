# PROV-125 — Cloud providers show empty model lists (slug ↔ models.dev-id key mismatch)

**Type:** Bug
**Area:** Rust native model selector — `codelet/sessions/src/cloud_models.rs`
**Reference (correct behaviour):** TS `src/tui/services/cloudSectionBuilder.ts`

---

## 1. Symptom

In the ratatui Model Selector, several cloud providers render as **empty rows**
(header shown, zero models) even though:

- The provider has valid credentials configured, AND
- models.dev **does** publish tool-capable models for that provider.

Confirmed victims (verified against live `https://models.dev/api.json` on 2026-07-04
and the on-disk cache `~/.fspec/cache/models.json`):

| Canonical slug (catalog.rs) | models.dev key (registry key) | tool-call models lost |
|-----------------------------|-------------------------------|-----------------------|
| `together`                  | `togetherai`                  | 25                    |
| `moonshot`                  | `moonshotai`                  | 9                     |

> Note: `zai` is **not** a victim — it exists verbatim as a models.dev key
> (14 tool-call models). An earlier hypothesis blaming `zai` was disproven by
> cross-checking the actual catalog keys.

---

## 2. The pipeline

1. **Fetch** — `codelet/providers/src/models/cache.rs`
   - `MODELS_DEV_URL = "https://models.dev/api.json"`.
   - Cached indefinitely at `{data_dir}/cache/models.json`; refetched only on
     missing/corrupt file or explicit `refresh()`.

2. **Parse** — `codelet/providers/src/models/types.rs`
   - `ModelsDevResponse` uses `#[serde(flatten)] providers: HashMap<String, ProviderInfo>`.
   - **The top-level JSON key IS the models.dev provider id** (`openai`, `google`,
     `togetherai`, `moonshotai`, …).

3. **Registry** — `codelet/providers/src/models/registry.rs`
   - `providers: HashMap<String, ProviderInfo>` keyed by that models.dev id.
   - `list_models(provider)` (line 142) does `self.providers.get(provider)` and
     returns `Err(ProviderError::config("registry", "Unknown provider: {provider}"))`
     on a key miss. **No slug aliasing inside the registry.**

4. **Selector population (Rust native path)** — `codelet/sessions/src/handle_impl.rs`
   `list_providers` (≈920-1008)
   - Starts from a **fixed** list `codelet/providers/src/catalog.rs::CANONICAL_PROVIDERS`
     (18 hand-written slugs), built-ins carry empty `models`.
   - For each built-in, fills models via
     `cloud_models.rs::cloud_model_entries(registry, slug, has_creds)`.

5. **The defective translation** — `codelet/sessions/src/cloud_models.rs`
   ```rust
   pub fn canonical_to_models_dev(canonical_id: &str) -> &str {
       match canonical_id {
           "gemini" => "google",
           other => other,          // <-- assumes slug == models.dev key
       }
   }
   ```
   ```rust
   let dev_id = canonical_to_models_dev(canonical_id);
   let models = match registry.list_models(dev_id) {
       Ok(models) => models,
       Err(_) => return Vec::new(),  // <-- silently swallows key miss
   };
   ```

---

## 3. Root cause

The registry is keyed by **models.dev ids**, but the Rust selector starts from a
**fixed canonical-slug list** and translates each slug back with a **single-rule
map** (`gemini → google`). Every canonical slug whose models.dev key differs and
is **not** covered by that one rule produces a `HashMap` key miss → `Err(_)` →
silent empty `Vec`.

Because the error is discarded (`Err(_) => Vec::new()`), a genuine data-mapping
bug is indistinguishable from a provider that legitimately has no models.

The TS reference is immune because `cloudSectionBuilder.ts` is **data-driven**:
it iterates the models.dev catalog itself (`allModels.map(...)`), so the bucket
key always exists — a provider can only be "empty" if it fails the credential
filter (in which case TS *drops* it) or all models fail the `toolCall` filter.

---

## 4. Divergence inventory (canonical slug → models.dev key)

Verified against the live catalog. `HIT` = key present; `MISS` = key absent.

```
openai         HIT   anthropic   HIT   cohere      HIT
gemini→google  HIT   mistral     HIT   xai         HIT
together       MISS  → real key "togetherai"  (BUG: 25 models lost)
huggingface    HIT   openrouter  HIT   groq        HIT
deepseek       HIT
moonshot       MISS  → real key "moonshotai"  (BUG: 9 models lost)
galadriel      MISS  → genuinely absent from models.dev (expected empty)
azure          HIT   zai         HIT
codex          MISS  → genuinely absent (handled by Codex synthesis elsewhere)
github-copilot HIT   (present, 22 tool-call models)
```

There are exactly **three** reasons a provider ends up empty:

1. **Slug ≠ models.dev key (THE DEFECT):** `together`, `moonshot`.
2. **Genuinely absent from models.dev (expected):** `galadriel`, `codex`.
   (`codex` is intended to be populated by the separate Codex re-parenting
   feature, not by this path.)
3. **No credentials configured:** `cloud_model_entries` returns empty when
   `has_credentials == false` (existing, intended gating).

---

## 5. Fix

### 5.1 Correct the mapping (required)
Extend `canonical_to_models_dev` to cover **every** confirmed divergence,
verified against the cached catalog keys (do not guess):

```rust
match canonical_id {
    "gemini"   => "google",
    "together" => "togetherai",
    "moonshot" => "moonshotai",
    other      => other,
}
```

### 5.2 Stop swallowing misses silently (required)
Replace `Err(_) => return Vec::new()` with logic that distinguishes an
**expected absence** from a **diagnosable divergence**:

- Maintain an explicit known-not-on-models.dev set: `codex`, `galadriel`.
  A miss for these → return empty **silently**. (`github-copilot` is **not** in
  this set: per section 4 it is a `HIT` in models.dev — key `github-copilot`
  equals its canonical slug — so it resolves normally when credentialed and a
  future miss for it stays diagnosable.)
- Any other slug that misses → `tracing::warn!` (the crate already uses
  `tracing`; see `session_manager.rs`, `handle_impl.rs`) then return empty.

This keeps the two rules from conflicting: expected-absent providers stay quiet,
but a NEW slug/key divergence becomes visible in logs instead of silently
producing an empty row.

> **Do NOT** use `println!`/`eprintln!` — production code. Use `tracing`.

### 5.3 (Considered, not required now) Data-driven path
The structural cure is to make the Rust path iterate registry keys like TS does,
eliminating the whole miss class. That is a larger refactor of `list_providers`
and out of scope for this bug; 5.1 + 5.2 fully resolve the reported defect and
make any future divergence loud.

---

## 6. Acceptance criteria (see example map on PROV-125)

- Credentialed `together` → shows `togetherai` tool-call models (non-empty).
- Credentialed `moonshot` → shows `moonshotai` tool-call models (non-empty).
- Credentialed `gemini` → still shows `google` models (regression guard).
- `codex` (known-absent) → empty, **no** warning.
- Credentialed slug that misses and is NOT known-absent → `tracing::warn!` + empty.

---

## 7. Test strategy

Unit-test in `codelet/sessions/tests/` (mirror existing
`rpc073_cloud_model_catalog.rs`):

- Construct a `ModelRegistry` from a fixture `ModelsDevResponse` whose keys are
  `togetherai`, `moonshotai`, `google`, each with ≥1 `tool_call` model.
- Assert `canonical_to_models_dev("together") == "togetherai"`,
  `("moonshot") == "moonshotai"`, `("gemini") == "google"`.
- Assert `cloud_model_entries(&registry, "together", true)` is non-empty and
  contains only tool-call, non-deprecated models, sorted newest-first.
- Assert `cloud_model_entries(&registry, "codex", true)` is empty.
- **No network calls** — build the registry from an in-memory fixture.

---

## 8. Files

- `codelet/sessions/src/cloud_models.rs` — fix target (mapping + miss handling).
- `codelet/providers/src/catalog.rs` — canonical slug source (read-only ref).
- `codelet/providers/src/models/registry.rs` — `list_models` behaviour (ref).
- `codelet/sessions/tests/rpc073_cloud_model_catalog.rs` — test pattern to mirror.
