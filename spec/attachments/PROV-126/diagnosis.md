# PROV-126 — Model selector cloud sections diverge from the TS reference

**Discovered while validating PROV-125.** PROV-125 fixed exactly one thing —
the pure slug→models.dev-key mapping (`together→togetherai`,
`moonshot→moonshotai`) — and its unit tests proved that mapping in isolation.
They never rendered the actual `/model` selector, so they could not catch the
**section-building** defects that produce "wrong models in the wrong areas."

## Pipeline map

- **Rust (under test):** `SessionManagerHandle::list_providers()`
  (`codelet/sessions/src/handle_impl.rs:920`) → `custom::list_providers_info()`
  (iterates `catalog.rs::CANONICAL_PROVIDERS`, 17 fixed slugs) → per-provider
  fill via `cloud_models::provider_has_credentials` + `cloud_model_entries`
  (`codelet/sessions/src/cloud_models.rs`) → local profile sections appended
  last (`handle_impl.rs:996`). The TUI view (`fspec-tui/.../model_selector/`)
  merely renders whatever sections this pipeline emits.
- **TS (correct reference):** `modelInitializationService.initializeModels()`
  → `loadCloudModels()` + `buildCloudSections()`
  (`src/tui/services/cloudSectionBuilder.ts:92`) → `extractCodexSection()`
  (`cloudSectionBuilder.ts:191`).

## The 6 discrepancies (each = a user-visible symptom)

### 1. No Codex-section synthesis / OpenAI re-parenting — ABSENT in Rust
TS forces `openai.hasCredentials = hasCodexOAuth || hasCodexApiKey`
(`cloudSectionBuilder.ts:117-119`), then `extractCodexSection` REMOVES the
`openai` section and rebuilds its models under a synthetic
`{ providerId: 'codex', providerName: 'Codex (ChatGPT)' }` pushed first
(`cloudSectionBuilder.ts:191-237, 153-155`). Rust has no equivalent anywhere;
`codex` is a static header in `catalog.rs:156-163` and is in
`KNOWN_ABSENT_FROM_MODELS_DEV`, so it always renders `(0 models)`.
**Symptom:** the "OpenAI API" section holds the full models.dev OpenAI catalog
that TS would relocate under "Codex (ChatGPT)"; "Codex (ChatGPT)" shows empty.

### 2. Empty/uncredentialed sections are NOT dropped — the most visible defect
TS keeps only `sectionsWithCreds.filter(s => s.hasCredentials)`
(`cloudSectionBuilder.ts:140`) and later `filter(s => s.models.length > 0)`
(`modelInitializationService.ts:200,238`). Rust `list_providers`
(`handle_impl.rs:958-997`) pushes EVERY canonical provider header
unconditionally; uncredentialed/known-absent ones just get an empty model Vec.
**Symptom:** the picker is full of dead "Provider (0 models)" rows (Cohere,
Mistral, xAI, Groq, DeepSeek, Galadriel, Azure, Z.AI, Codex, GitHub Copilot, …).

### 3. Provider order inverted
TS: `[...profileSections, ...customSections, ...cloudSections]`
(`modelInitializationService.ts:196-200`). Rust: cloud/canonical first, customs,
then local profiles LAST (`handle_impl.rs:996`). Because both pick "first
section with models" as the default, the auto-selected default model differs.

### 4. Split credential gating
The header `available` flag (`list_providers_info` → `has_openai`/`has_codex`/
`has_github_copilot`, which DO check OAuth files) is decoupled from the
model-population gate (`list_providers` → `provider_has_credentials` →
`resolve_credential`, which honors OAuth ONLY for anthropic —
`credentials/resolver.rs:158-163`). **Symptom:** OAuth-only providers (codex,
github-copilot) and Codex-authed OpenAI can show a credentialed header with zero
models.

### 5. No Codex allowlist filtering
TS applies `filterByCodexAllowlist` (`cloudSectionBuilder.ts:210-221`) to the
OpenAI→Codex models. Rust applies none.

### 6. Codex OAuth login yields zero selectable models
Consequence of #1 + #4: with `~/.codex/auth.json` but no `OPENAI_API_KEY`, TS
shows a populated "Codex (ChatGPT)"; Rust shows empty "OpenAI API" AND empty
"Codex (ChatGPT)".

## Regression net

`e2e/prov-126-cloud-sections.test.ts` + `e2e/fixtures/prov126-models.json`.
Seeds a throwaway HOME with a models.dev cache (keys: openai, anthropic,
togetherai, moonshotai, cohere, mistral) and ENV credentials for ONLY
openai/anthropic/together/moonshot. Opens `/model` and asserts TS parity:
credentialed sections populate; cohere/mistral (catalogued but uncredentialed)
are DROPPED (no "(0 models)" headers). Currently FAILS on discrepancy #2.

Full rendered buffer captured at `/tmp/prov126_model_view.txt`.

## CONFIRMED — real rendered buffer (e2e run against the built binary)

Binary built with `--features test-stub-provider`; `/model` opened live:

```
Select Model (5 models)
> ▶ OpenAI API (2 models)
  ▶ Anthropic (1 models)
  ▶ Cohere (0 models)            ← dead
  ▶ Google Gemini (0 models)     ← dead
  ▶ Mistral AI (0 models)        ← dead
  ▶ xAI (0 models)               ← dead
  ▶ Together AI (1 models)       ← PROV-125 fix working
  ▶ Hugging Face (0 models)      ← dead
  ▶ OpenRouter (0 models)        ← dead
  ▶ Groq (0 models)              ← dead
  ▶ DeepSeek (0 models)          ← dead
  ▶ Moonshot (1 models)          ← PROV-125 fix working
  ▶ Galadriel (0 models)         ← dead
  ▶ Azure OpenAI (0 models)      ← dead
  ▶ Z.AI (0 models)              ← dead
  ▶ Codex (ChatGPT) (0 models)   ← dead (should hold the OpenAI catalog)
  ▶ GitHub Copilot (0 models)    ← dead
```

**13 dead "(0 models)" headers.** Test FAILED at the zero-model-headers guard
(first trip: `/Cohere \(0 models\)/`). The 4 "populated section" assertions
PASSED (OpenAI API 2, Anthropic 1, Together AI 1, Moonshot 1) — so PROV-125's
slug mapping is confirmed working live, and the remaining defect is purely the
section-building layer (discrepancy #2 dominant, plus Codex #1 visible as the
empty "Codex (ChatGPT)" while "OpenAI API" holds the catalog).

## Recommended split (each independently shippable, TS-parity target)
- **PROV-126a** Drop empty/uncredentialed cloud sections (#2). Highest impact,
  lowest risk. Fixes the wall of "(0 models)" rows.
- **PROV-126b** Codex-section synthesis + OpenAI re-parenting + allowlist
  (#1, #5, #6).
- **PROV-126c** Unify credential gating so header availability and model
  population agree (#4).
- **PROV-126d** Provider ordering parity + default-model selection (#3).
