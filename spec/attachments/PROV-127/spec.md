# PROV-127 — Drop empty/uncredentialed cloud sections from the model selector

**Parent:** PROV-126 · **Discrepancy #2** (highest impact, lowest risk) · **Type:** bug

## Problem

The Rust `/model` selector renders **every** canonical cloud provider as a section
header, even when the provider has no credentials and/or no models. The result is a
wall of dead `Provider (0 models)` rows.

Live-captured buffer (built with `--features test-stub-provider`, ENV creds for only
openai/anthropic/together/moonshot):

```
Select Model (5 models)
> ▶ OpenAI API (2 models)
  ▶ Anthropic (1 models)
  ▶ Cohere (0 models)            ← dead
  ▶ Google Gemini (0 models)     ← dead
  ▶ Mistral AI (0 models)        ← dead
  ▶ xAI (0 models)               ← dead
  ▶ Together AI (1 models)
  ▶ Hugging Face (0 models)      ← dead
  ▶ OpenRouter (0 models)        ← dead
  ▶ Groq (0 models)              ← dead
  ▶ DeepSeek (0 models)          ← dead
  ▶ Moonshot (1 models)
  ▶ Galadriel (0 models)         ← dead
  ▶ Azure OpenAI (0 models)      ← dead
  ▶ Z.AI (0 models)              ← dead
  ▶ Codex (ChatGPT) (0 models)   ← dead
  ▶ GitHub Copilot (0 models)    ← dead
```

**13 dead headers.**

## Root cause

`SessionManagerHandle::list_providers()` (`codelet/sessions/src/handle_impl.rs`,
around lines 958–997) iterates `catalog.rs::CANONICAL_PROVIDERS` (17 fixed slugs) and
pushes a header for **every** provider, giving uncredentialed/known-absent ones an
empty model `Vec` instead of omitting them.

## TS reference (correct behaviour)

- `cloudSectionBuilder.ts:140` — keeps only `sectionsWithCreds.filter(s => s.hasCredentials)`.
- `modelInitializationService.ts:200,238` — later `filter(s => s.models.length > 0)`.

Net TS behaviour: a cloud section is rendered **only if** it has credentials AND at
least one model.

## Scope of THIS card

1. Drop any cloud section whose model list is empty (`models.length === 0`) so it does
   not render as a header. This alone clears all 13 dead rows in the fixture.
2. Preserve populated sections exactly (OpenAI API, Anthropic, Together AI, Moonshot).
3. Fix the `(0 models)` / `(1 models)` pluralization in `rows.rs:76-80` and
   `state.rs:177` (defensive — after dropping empties, `(1 models)` is still wrong).

**Out of scope (later cards):** Codex re-parenting (PROV-129), unifying the split
credential gate (PROV-128), section ordering/default (PROV-130). This card deliberately
drops on `models.length === 0`, which is credential-agnostic and safe.

## Acceptance criteria (example-map seeds)

- **Rule:** A cloud provider section with zero models is not rendered in the selector.
- **Rule:** A cloud provider section with one or more models is rendered unchanged.
- **Rule:** Section header model counts are pluralized correctly (`(1 model)`, `(2 models)`).
- **Example:** Fixture with creds for openai/anthropic/together/moonshot renders exactly
  those four cloud sections; cohere/mistral (catalogued, uncredentialed) are dropped.
- **Example:** A provider with exactly one model shows `(1 model)` not `(1 models)`.

## Regression net

`e2e/prov-126-cloud-sections.test.ts` + `e2e/fixtures/prov126-models.json` — currently
FAILS at the first `not.toMatch(/Cohere \(0 models\)/)` guard. This card must flip that
test green (except any assertions specific to Codex re-parenting, which belong to
PROV-129 — coordinate the shared e2e so PROV-127 owns the "no dead sections" assertions).

## Key files

- `codelet/sessions/src/handle_impl.rs` — `list_providers()` (section assembly).
- `codelet/sessions/src/cloud_models.rs` — `cloud_model_entries`, `provider_has_credentials`.
- `codelet/sessions/src/catalog.rs` — `CANONICAL_PROVIDERS`, `KNOWN_ABSENT_FROM_MODELS_DEV`.
- `codelet/fspec-tui/src/views/model_selector/rows.rs:76-80`, `state.rs:177` — pluralization.
- TS reference: `src/tui/services/cloudSectionBuilder.ts`, `src/tui/services/modelInitializationService.ts`.
