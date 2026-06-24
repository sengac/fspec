# PROV-120 — Restore TS-parity first-available model initialization removed by PROV-101

**Status:** specifying
**Epic:** provider-settings-parity
**Depends on / corrects:** PROV-101 (also relates to PROV-117, PROV-118, PROV-119, MODEL-006)
**Type:** Corrective (regression introduced by an over-broad removal)

---

## 1. Summary

PROV-101 ("Remove all provider/model/profile selection fallbacks") **over-removed**. It
correctly deleted the hardcoded `anthropic/claude-opus-4-5` substitution, but it *also*
deleted the legitimate **first-available model selection** that the TypeScript reference
performs at startup (`initializeModels()` / `selectDefaultModel()`).

Because the whole project is a **port from TypeScript to Rust, where the TypeScript
implementation is the source of truth for intended behavior**, removing first-available
selection was a parity regression — not a cleanup.

The observable result: on **every fresh launch** the Rust port has `default_model = None`,
so the bootstrap `create_session` is declined:

```
ERROR codelet_sessions::handle_impl: create_session declined:
  no default model set (PROV-101: no anthropic fallback)
```

In the TS reference this does **not** happen on a normal launch, because a model is
resolved (persisted → else first-available) *before* the session is created.

---

## 2. How this was discovered

Investigating `~/.fspec/logs/fspec-combined.log.2026-06-24`:

- **3× ERROR** `create_session declined: no default model set (PROV-101: no anthropic fallback)`
- **6× WARN** `probe_profile_models: /v1/models probe failed; marking profile unreachable`
  for `http://localhost:8080` and `http://192.168.0.50:8000`

The WARNs are an environmental symptom (two local OpenAI-compatible servers were down).
In the **TS reference** that is harmless: unreachable, zero-model sections are filtered
out and selection falls through to the next reachable section (e.g. a cloud provider).
In the **Rust port** there is no fall-through, because the first-available step itself was
removed — so a down-local-profile environment leaves the default permanently `None`.

---

## 3. The core mistake: two different things both called "fallback"

PROV-101 conflated two distinct behaviors:

| # | Behavior | In TS reference? | Correct action |
|---|----------|------------------|----------------|
| ① | Hardcoded `get_default_model().unwrap_or_else(\|\| "anthropic/claude-opus-4-5")` | **No** | ✅ Remove (PROV-101 did this correctly) |
| ② | First-available selection: persisted → else first reachable model-bearing section | **Yes** (`selectDefaultModel`) | ❌ Must NOT remove — this is intended behavior |
| ③ | Provider-priority chain `Claude>Gemini>ZAI>Codex>Copilot>OpenAI` | **No** | ✅ Remove (PROV-101 did this correctly) |

PROV-101 removed ①, ②, **and** ③. Only ① and ③ were genuine "silent fallbacks" absent
from TS. ② is the legitimate resolution path and must be restored.

> Note: ② is **not** the same as ③. First-available iterates over *reachable,
> model-bearing sections in build order (profiles → custom → cloud)* and takes the first
> model. It is **not** a hardcoded provider preference list. Removing ③ was right;
> removing ② was the error.

---

## 4. TypeScript reference behavior (source of truth)

File: `src/tui/services/modelInitializationService.ts`

### `initializeModels()` — runs at startup, before any session is created
1. Build sections in parallel: **profiles → custom → cloud**
   (`loadProfileSections()`, `loadCustomProviderSections()`, `buildCloudSections()`).
2. Filter out sections that are `isUnreachable` AND have `models.length === 0`.
3. Read persisted model string `tui.lastUsedModel` from `~/.fspec/fspec-config.json`
   (`loadPersistedModelString()`).
4. **Restore** persisted model if it matches a section with `hasCredentials === true`
   and the model exists (`restorePersistedModel()`).
5. **Else first-available** (`selectDefaultModel()`):
   ```ts
   for (const section of sections) {
     if (section.models.length > 0) {
       return { section, model: section.models[0] }; // first reachable model
     }
   }
   return null;
   ```
6. If still nothing → `currentModel = null` (genuine empty state — no anthropic).

### Consequence
`create_session` is only ever called with `currentModel` already populated on a normal
launch. The "require explicit model / decline" path is a **true edge case** (zero reachable
models anywhere), not the default experience.

---

## 5. Current Rust behavior (the regression)

- `codelet/sessions/src/handle_impl.rs:82` — `create_session` reads `get_default_model()`;
  `None` → hard decline (empty `SessionId`). No anthropic substitution. (Correct per ①.)
- The default is **only** ever set reactively, when the user manually opens `/model` and
  picks — `handle_model_selected_no_session` in
  `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs` (the only non-test caller
  of `set_default_model`).
- There is **no startup init step** that runs first-available before bootstrap
  `create_session`. `grep` for `lastUsedModel` / `initializeModels` / `selectDefaultModel`
  in `codelet/fspec-tui` and `codelet/tui` returns **zero** hits.
- PROV-119 persists/loads an explicitly-chosen default to `default-model.json`, but on a
  fresh install nothing was ever chosen → `None` → decline. PROV-119 even encodes this as
  intended:
  > *Scenario: First launch with no persisted config has no default model — first
  > create_session declined until selection.*

So the Rust port faithfully implements PROV-101/119 — but those cards themselves encoded
the wrong (non-parity) behavior for ②.

---

## 6. Required change (what "done properly" looks like)

Add a **startup model-initialization step** to the Rust port that mirrors
`initializeModels()`:

1. Build sections via the existing Rust path
   (`codelet/sessions/src/profile_sections.rs` + `cloud_model_entries` / `list_providers`).
2. Filter out unreachable + zero-model sections.
3. Resolve: **persisted (restore) → else first-available reachable model**.
4. Commit via `set_default_model(...)` **before** the bootstrap `create_session`.

### Preserve (do NOT walk back)
- ① No hardcoded `anthropic/claude-opus-4-5` substitution in `handle_impl`.
- ③ `resolve_unambiguous_provider` — ambiguous multi-provider credential state → `Err`
  (no silent Claude pick).
- The decline path remains for the **genuine** zero-reachable-models edge case.

### Net effect
The decline (`create_session declined: no default model set`) becomes a *real edge case*
(no reachable models at all), exactly as in TS — instead of firing on every normal launch.

---

## 7. Acceptance criteria (rules)

1. Startup model resolution order: restore persisted (`tui.lastUsedModel`) if it matches a
   credentialed section, ELSE first-available reachable model with ≥1 model.
2. First-available = first section in order **profiles → custom → cloud** that has ≥1
   model; take its first model.
3. Resolved model committed via `set_default_model` **before** the bootstrap
   `create_session`.
4. Sections that are unreachable AND have zero models are excluded from selection.
5. PRESERVE PROV-101: no hardcoded anthropic/claude substitution.
6. PRESERVE PROV-101: no silent pick on ambiguous multi-provider credential state;
   first-available is over reachable model-bearing sections, not a provider-priority chain.
7. Genuine zero reachable model-bearing sections → no default set, `create_session`
   declines (PROV-101 edge case preserved) — not the normal-launch path.

---

## 8. Worked examples (green cards)

- Fresh launch, no persisted model, one reachable cloud provider with models → startup
  selects that provider's first model; `create_session` succeeds without opening `/model`.
- Persisted `tui.lastUsedModel` still matches a credentialed section → restore exactly
  that model (not first-available).
- Persisted model's provider lost credentials / model gone → fall back to first-available.
- Two local profiles unreachable (zero models) + reachable cloud provider → skip profiles,
  select cloud model as first-available.
- Reachable local profile with models AND reachable cloud both exist → first-available
  picks the profile model (profiles precede cloud).
- Genuinely zero reachable model-bearing sections → no default committed; `create_session`
  declines with empty `SessionId` (PROV-101 edge case preserved).
- Anthropic not credentialed and not the only reachable provider → never substitute
  anthropic/claude as a hardcoded default.

---

## 9. Open questions (red cards)

- **Q1 (persistence store):** Should Rust read/restore from `tui.lastUsedModel` in
  `fspec-config.json` for true TS parity, or keep PROV-119's `default-model.json` — and
  which is source of truth?
- **Q2 (placement):** Where should startup init run — Rust combined-mode bootstrap
  (`rpc-server`) before bootstrap `create_session`, or the `fspec-tui` app mount path
  mirroring TS `AgentView`? Must it also cover headless/CLI session creation?
- **Q3 (selector interaction):** Does `model-selector-no-auto-select.feature`
  (PROV-101 #4/#5) stay unchanged (selector shows nothing pre-selected) while startup sets
  the default separately, or does restoring a default also highlight the selector cursor?

---

## 10. Key references

- TS: `src/tui/services/modelInitializationService.ts`
  (`initializeModels`, `selectDefaultModel`, `restorePersistedModel`,
  `loadPersistedModelString`)
- TS: `src/tui/services/profileSectionBuilder.ts` (`loadProfileSections`, `/v1/models` probe)
- TS: `src/tui/services/modelSelectionService.ts` (persists `tui.lastUsedModel`)
- Rust: `codelet/sessions/src/handle_impl.rs` (`create_session` decline)
- Rust: `codelet/sessions/src/session_manager.rs` (`get_default_model` / `set_default_model`)
- Rust: `codelet/sessions/src/default_model_persistence.rs` (`default-model.json`)
- Rust: `codelet/sessions/src/profile_sections.rs` (`probe_profile_models`)
- Rust: `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs` (reactive set_default_model)
- PROV-101 features (to reconcile): `spec/features/session-creation-requires-explicit-model.feature`,
  `spec/features/provider-resolution-no-silent-default.feature`,
  `spec/features/model-selector-no-auto-select.feature`

---

## 11. Resolved questions (answered from the TypeScript reference)

All three open questions were answered by tracing the TS reference under `src/tui/`.

### A1 — Persistence store (Q1): use `tui.lastUsedModel` in `fspec-config.json`
`tui.lastUsedModel` in `~/.fspec/fspec-config.json` (user scope) is the **single source of
truth** in TS.

- **READ:** `modelInitializationService.ts:73` — `loadPersistedModelString()` →
  `loadConfig()` → `config.tui.lastUsedModel`. `loadConfig()` (`src/utils/config.ts:79`)
  deep-merges user (`~/.fspec/fspec-config.json`) + project (`spec/fspec-config.json`),
  project overriding.
- **WRITE:** `modelSelectionService.ts:179-186` — `selectModel()` →
  `writeConfig('user', { tui: { lastUsedModel: modelString } })`, written **only when the
  session update succeeded**. The value is a composite model string from
  `buildModelString(...)` (e.g. `anthropic/claude-opus-4`, or profile-qualified
  `openai:work-vllm/Qwen/Qwen3-80B`).

**Decision:** For parity, Rust must read/restore from `fspec-config.json` →
`tui.lastUsedModel`. PROV-119's `default-model.json:model` **diverges** from TS and should
be reconciled so `fspec-config.json` `tui.lastUsedModel` is the source of truth. (This is a
follow-on reconciliation against PROV-119.)

### A2 — Placement (Q2): TUI app-mount startup, before bootstrap `create_session`
- `initializeModels()` runs on **AgentView mount** (`AgentView.tsx:1469-1536`, call at
  line 1500), **once**, idempotent via the `modelsInitialized` guard
  (`modelInitializationService.ts:168-181`).
- It runs **before** session creation. Session creation (`AgentView.tsx:3772`
  `createSession(...)`) happens later and **receives** the already-resolved model;
  `sessionService.createSession` (`sessionService.ts:120`) does **not** initialize models.
- There is **no headless/CLI model-init path** — it is purely a TUI concern.
  (`commands/reverse.ts` "session" is unrelated ACDD workflow state.)

**Decision:** Place the Rust init step in the **fspec-tui app startup/mount path**, running
before the bootstrap `create_session`. Do **not** add it to a headless `rpc-server` path.

### A3 — Selector interaction (Q3): `model-selector-no-auto-select.feature` stays unchanged
- The selector seeds its cursor from `currentModel`: `ModelSelectorScreen.tsx:94-119`
  auto-expands the section and **highlights the row** matching `currentModel.apiModelId`.
- If there is **no** current model, the effect returns early (line 95): nothing
  highlighted, cursor on the first section header, `selectedModelIdx = -1` — exactly the
  PROV-101 no-auto-select invariant.

**Decision:** PROV-101's `model-selector-no-auto-select.feature` is **not contradicted** and
stays unchanged. PROV-120 simply means `currentModel` is usually non-null at startup, so the
selector legitimately highlights the resolved default. The "nothing selected when no current
model" invariant is preserved.

### Net implication for the existing PROV-101 feature files
- `model-selector-no-auto-select.feature` — **no change** (invariant preserved).
- `provider-resolution-no-silent-default.feature` — **no change** (ambiguous multi-cred
  still errors; first-available is over reachable sections, not a provider-priority chain).
- `session-creation-requires-explicit-model.feature` — **semantics preserved**, but the
  decline becomes the genuine zero-reachable-models edge case rather than the normal-launch
  path. Its scenarios remain valid (they construct a SessionManager with no default
  directly); PROV-120 adds the startup step that ensures a default is normally present.

---

## 12. Scope decision: fold PROV-119 store reconciliation into PROV-120

Per A1, `tui.lastUsedModel` in `~/.fspec/fspec-config.json` is the TS source of truth, which
means **PROV-119's separate `default-model.json` is itself a non-parity divergence**. Rather
than split a second corrective card, this reconciliation is **folded into PROV-120** (same
root cause, same code paths):

- **Restore/read:** startup init reads `tui.lastUsedModel` from `fspec-config.json`
  (user-merged-with-project) — matching `loadPersistedModelString()`.
- **Persist/write:** model selection writes `tui.lastUsedModel` to the **user**
  `fspec-config.json` — matching `selectModel()` / `writeConfig('user', …)`.
- **Back-compat migration:** if `default-model.json` exists and `fspec-config.json` has no
  `tui.lastUsedModel`, read it once for continuity; **new writes go to `fspec-config.json`**.

This subsumes the storage half of PROV-119 while preserving its persistence intent
(a chosen default survives restarts) — now via the TS-parity location.
