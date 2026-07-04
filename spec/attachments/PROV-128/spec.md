# PROV-128 — Unify credential gating between section availability and model population

**Parent:** PROV-126 · **Discrepancy #4** · **Type:** bug · **Depends on:** PROV-127

## Problem

Rust has **two independent credential checks** that disagree:

1. **Header `available` flag** — `list_providers_info()` computes
   `has_openai` / `has_codex` / `has_github_copilot`, and these DO consult the OAuth
   credential files (`~/.codex/auth.json`, Copilot token, Claude OAuth).
2. **Model population gate** — `list_providers()` calls
   `cloud_models::provider_has_credentials` → `resolve_credential`
   (`codelet/credentials/resolver.rs:158-163`), which honors OAuth **only for
   anthropic**.

**Symptom:** OAuth-only providers (codex, github-copilot) and Codex-authed OpenAI can
show a *credentialed header with zero models* — the header believes creds exist, but the
model-population path refuses to resolve the OAuth credential, so no models load.

## TS reference (correct behaviour)

TS derives a **single** `hasCredentials` per section and uses it for both "keep the
section" and "populate the section". `cloudSectionBuilder.ts:117-119` explicitly folds
OAuth into the flag: `openai.hasCredentials = hasCodexOAuth || hasCodexApiKey`. There is
one source of truth.

## Scope of THIS card

1. Make `resolve_credential` (or the `provider_has_credentials` path used by
   `list_providers`) honor OAuth credentials for **all** OAuth-capable providers
   (codex, github-copilot, anthropic) — not anthropic-only.
2. Ensure the header `available` flag and the model-population gate read from the **same**
   credential resolution, so a provider is either credentialed-and-populated OR dropped —
   never credentialed-but-empty.
3. This makes PROV-127's "drop empty sections" behaviour also correctly drop
   headers-without-resolvable-creds, and unblocks PROV-129 (Codex synthesis needs the
   OAuth credential to actually resolve OpenAI models).

**Out of scope:** the actual Codex re-parenting/allowlist (PROV-129), ordering (PROV-130).

## Acceptance criteria (example-map seeds)

- **Rule:** Credential resolution honors OAuth for every OAuth-capable provider
  (anthropic, codex, github-copilot), not anthropic only.
- **Rule:** A provider's header-availability and its model-population use the same
  credential decision — no credentialed-but-empty sections.
- **Example:** With a Codex OAuth file present and no `OPENAI_API_KEY`, the credential
  gate resolves OpenAI/Codex credentials (models can populate).
- **Example:** With a GitHub Copilot OAuth token present, the Copilot credential resolves
  and the section populates (or is dropped consistently if no models).
- **Example:** A provider with no credential of any kind resolves to "no credential" in
  BOTH the header flag and the population gate.

## Key files

- `codelet/credentials/resolver.rs:158-163` — `resolve_credential` OAuth branch (currently anthropic-only).
- `codelet/sessions/src/cloud_models.rs` — `provider_has_credentials`.
- `codelet/sessions/src/handle_impl.rs` — `list_providers()` + `list_providers_info()`.
- TS reference: `src/tui/services/cloudSectionBuilder.ts:117-119,140`.
