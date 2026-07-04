# PROV-129 — Synthesize the Codex (ChatGPT) section (re-parent + allowlist OpenAI models)

**Parent:** PROV-126 · **Discrepancies #1, #5, #6** · **Type:** bug · **Depends on:** PROV-128

## Problem

Rust performs **no** Codex-section synthesis. `codex` is a static header in
`catalog.rs:156-163` and is listed in `KNOWN_ABSENT_FROM_MODELS_DEV`, so it always
renders `Codex (ChatGPT) (0 models)`. Meanwhile the full models.dev OpenAI catalog stays
under the `OpenAI API` section.

**Symptoms:**
- **#1** OpenAI API section holds models that TS relocates under Codex (ChatGPT).
- **#5** No Codex allowlist filtering of OpenAI models.
- **#6** Codex OAuth login (`~/.codex/auth.json`, no `OPENAI_API_KEY`) yields zero
  selectable models — both OpenAI API and Codex (ChatGPT) render empty.

## TS reference (correct behaviour)

1. `cloudSectionBuilder.ts:117-119` — `openai.hasCredentials = hasCodexOAuth || hasCodexApiKey`.
2. `cloudSectionBuilder.ts:191-237` — `extractCodexSection()`:
   - REMOVES the `openai` section,
   - rebuilds its models under a synthetic
     `{ providerId: 'codex', providerName: 'Codex (ChatGPT)' }`,
   - pushes that section **first** (`:153-155`),
   - applies `filterByCodexAllowlist` (`:210-221`).

So when Codex creds are active, users see a populated **Codex (ChatGPT)** section (the
allowlisted OpenAI models) and **no** standalone OpenAI API section.

## Scope of THIS card

1. When Codex credentials (OAuth or API key) are present, re-parent the OpenAI cloud
   models under a synthetic `Codex (ChatGPT)` section.
2. Apply the Codex allowlist filter to those models (port `filterByCodexAllowlist`).
3. Remove/omit the standalone OpenAI API section in that case (avoid duplication).
4. When Codex creds are absent but a plain `OPENAI_API_KEY` is present, keep the normal
   OpenAI API section (no re-parenting) — TS parity.
5. Depends on PROV-128 so the Codex OAuth credential actually resolves to models.

**Out of scope:** dropping empties (PROV-127), ordering/default (PROV-130) — though the
Codex section's "pushed first" positioning is reconciled in PROV-130.

## Acceptance criteria (example-map seeds)

- **Rule:** When Codex OAuth or Codex API key is present, OpenAI cloud models are
  re-parented under a single `Codex (ChatGPT)` section.
- **Rule:** The re-parented model list is filtered by the Codex allowlist.
- **Rule:** When the Codex section is synthesized, the standalone OpenAI API section is
  not rendered.
- **Rule:** With only `OPENAI_API_KEY` (no Codex creds), the normal OpenAI API section is
  shown and no Codex re-parenting occurs.
- **Example:** Codex OAuth file present, no `OPENAI_API_KEY` → `Codex (ChatGPT)` shows the
  allowlisted models; `OpenAI API` is absent. (Fixes #6.)
- **Example:** A models.dev OpenAI model not in the allowlist is excluded from Codex (ChatGPT).

## Regression net

Extend `e2e/prov-126-cloud-sections.test.ts` with a Codex-OAuth fixture variant asserting
`Codex (ChatGPT)` is populated and `OpenAI API` is absent.

## Key files

- `codelet/sessions/src/handle_impl.rs` — `list_providers()` section assembly.
- `codelet/sessions/src/cloud_models.rs` — OpenAI model entries + allowlist.
- `codelet/sessions/src/catalog.rs:156-163` — static `codex` header, `KNOWN_ABSENT_FROM_MODELS_DEV`.
- TS reference: `src/tui/services/cloudSectionBuilder.ts:117-119,153-155,191-237,210-221`.
