# RPC-343 — Model selector selection drops rich model metadata

**Severity: HIGH** (upgraded from the initial LOW-MEDIUM estimate). The headline
finding is NOT merely "metadata dropped on the wire" — the Rust backend
**re-resolves NOTHING** on the mid-session `set_model` path, so the model change
silently degrades to "rename the displayed label only."

## Summary

TS builds a rich 12-field `ModelSelection` object on selection and persists it
whole to the session manifest, so all fields survive. Rust emits
`Action::ModelSelected(session_id, provider_key, model_id)` — three strings — and
the backend `set_model` swaps two `String` fields **without re-resolving**
context window, max output, compaction threshold, facade, or reasoning, and
**without updating the inner provider manager** that actually issues requests.

---

## PART 1 — The TS `ModelSelection` object

Built in `selectModel` — `src/tui/hooks/useModelSelectorState.ts:253-274`.
Type: `src/tui/types/provider.ts:51-87`. Persisted whole to the store
(`src/tui/store/modelStore.ts:115-118`) → session manifest.

| Field | Source (line) | Behavioral vs cosmetic |
|---|---|---|
| `providerId` | `section.providerId` (`:259`) | **Behavioral** — provider/endpoint |
| `modelId` | `extractModelIdForRegistry(model.id)` (`:260`) | **Behavioral** — registry key |
| `apiModelId` | `model.id` (`:261`) | **Behavioral** — literal API model string |
| `displayName` | `model.name` (`:262`) | Cosmetic |
| `reasoning` | `model.reasoning` (`:263`) | **Behavioral** — thinking/request shaping |
| `hasVision` | `model.hasVision` (`:264`) | Mostly cosmetic |
| `contextWindow` | `model.contextWindow` (`:265`) | **Behavioral** — compaction/token budget |
| `maxOutput` | `model.maxOutput` (`:266`) | **Behavioral** — output cap |
| `profileName` | `section.profileName` (`:267`) | **Behavioral (local profiles)** |
| `profileConfig` | `section.profileConfig` (`:268`) | **Behavioral** — base_url/api_style/key |
| `facade` | `lookupFacadeOverride(section, model.id)` (`:255,269`) | **Behavioral** — tool-schema dialect |
| `compactionThreshold` | `lookupCompactionThreshold(...)` (`:256,270`) | **Behavioral** — compaction trigger |

---

## PART 2 — The Rust path (only 3 strings cross the boundary)

- Emission: `views/model_selector/mod.rs:281-285`
  - `Action::ModelSelected(session_id, row.provider_key.clone(), row.model_id.clone())`
- Action type (3-tuple of strings): `components/mod.rs:392`
  - `ModelSelected(SessionId, String, String)`
- Dispatch: `app/dispatch_model_thinking_dialogs.rs:217-218` → `handle_model_selected` `:87-114`
  - `:95-96` cache model_id for the `(current)` marker (cosmetic)
  - `:104-106` `backend.set_session_model(session_id, provider_id, model_id)`
  - `:109-110` re-fetch `get_model_info` to repaint header
- Backend chain:
  - `transport/embedded.rs:198-209` `set_session_model(SessionId, String, String)`
  - `rpc/src/lib.rs:1012-1026` → `handle.set_model(&session_id, &provider_id, &model_id)`
  - `sessions/src/handle_impl.rs:1008-1022` → `session.set_model(Some(provider_id), Some(model_id))`
  - `background_session.rs:722-725` — **only swaps two `String` fields**

### Does the backend RE-RESOLVE metadata? NO. (headline)
- `set_model_limits()` (writes `cached_context_window`,
  `cached_max_output_tokens`, `cached_compaction_threshold`) — `background_session.rs:729-733`
  — is **only called at session creation** (`session_manager.rs:557`, `:803`),
  **never from `set_model`**.
- `provider_manager.select_model()` / `set_model_direct()` (re-resolve
  `context_window()`, `max_output_tokens()`, `facade_override()`) — **only at
  creation** (`session_manager.rs:485-506, 739`). The inner
  `codelet_cli::session::Session` (built `from_provider_manager`,
  `session_manager.rs:513`) that issues requests via `send_input`
  (`handle_impl.rs:104-120`) is **untouched** by `set_model`.

Consequences:
- `get_session_model` (`handle_impl.rs:180-211`) returns the new id strings but
  the **stale cached context_window/max_output/compaction_threshold** of the
  PREVIOUS model.
- The change does not propagate to the inner provider manager at all → next
  turn's API model / facade / reasoning may be stale.

---

## PART 3 — Lost vs. safely re-resolvable

Safely re-resolvable server-side from `provider_id + model_id` (live in the
provider registry / ProviderManager — the 3-tuple is sufficient IF the backend
bothered to re-resolve):
- `displayName`, `reasoning`, `hasVision`, `contextWindow`, `maxOutput`,
  `facade`, `compactionThreshold`, `apiModelId`
- Providers: `providers/src/manager.rs:437` (select_model), `:1080`
  (context_window), `:1152` (facade_override)

Genuinely lost / not reconstructible from provider+model id alone:
- `profileConfig` for **local-profile / custom-provider** selections (base_url,
  api_style, api_key_env_var — `provider.ts:79`), and any per-selection
  facade/compactionThreshold override originating from custom-model config the
  backend doesn't independently re-read on this path.

**But the dominant problem is that the backend re-resolves NOTHING — even
trivially re-resolvable fields are left stale.**

---

## PART 4 — Recommendation

**(A) PREFERRED — make the backend re-resolve on `set_model`** (minimal wire change).
In `handle_impl.rs:1008-1022`, after swapping the strings:
1. Re-run creation-time resolution — `provider_manager.select_model(...)` /
   `set_model_direct(...)` on the inner session;
2. `session.set_model_limits(context_window, max_output_tokens, resolve_compaction_threshold(...))`
   (mirror `session_manager.rs:510-561`);
3. Re-derive facade via `provider_manager.facade_override()`.

This fixes displayName/reasoning/hasVision/contextWindow/maxOutput/facade/
compactionThreshold after a mid-session switch with **no action-signature change**.

**(B) Only if profile/custom overrides prove non-reconstructible** — widen
`Action::ModelSelected` (`components/mod.rs:392`) and `set_session_model`
(`rpc/src/lib.rs:1012`, transports, tests) to carry
`SelectionMetadata { profile_config, facade_override, compaction_threshold }`.
NOTE: this touches the locked source-shape contract test
`codelet/fspec-tui/tests/source_shape_rpc022.rs:93-97` (asserts the exact 3-arg
signature) — must be updated in lockstep.

---

## Severity assessment: HIGH
- Compaction triggers at the WRONG threshold (stale) → context mismanagement,
  possible over-budget API errors or premature compaction.
- Token-budget math uses the PREVIOUS model's context_window/max_output.
- facade/reasoning not re-derived → wrong tool-schema dialect / thinking config.
- Because the inner Session/ProviderManager is never updated, the switch may not
  even take effect for the next request beyond the cosmetic header.

Fix (A) is low-effort and closes the gap for all re-resolvable fields.
