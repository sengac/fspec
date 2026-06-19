# RPC-344 — Model selector missing custom-model CRUD (a/e/d keybinds)

**Severity: LOW (UI parity) but with a BLOCKING backend dependency.**
The TS model selector has a complete, fully wired custom-model CRUD subsystem
(MODEL-004). Rust has the READ half only (the `[C]` badge renders). The entire
backend WRITE surface (persistence + RPC) **does not exist in Rust** and must be
built first.

## Verdict

Rust can *read* custom models but has no way to *create/update/delete* them. This
is not just missing keybinds — the persistence layer, wire types, RPC methods,
NAPI bindings, and Action variants are all absent. **Gating prerequisite:** build
the backend custom-model surface before any UI work is useful.

---

## PART 1 — TS reference (fully wired)

### Keybinds — `src/tui/components/ModelSelectorScreen.tsx:149-187`
All gated on the focused section being a profile section (`sec.profileName`):
- `a` add (`:150-158`): profile-header only → mode `add-custom-model`, clears form
- `e` edit (`:161-175`): requires profile + `selectedModelIdx>=0` + id in
  `sec.customModelIds` + matching `customDef` → mode `edit-custom-model`, prefills
- `d` delete (`:178-187`): same guard → mode `delete-custom-model-confirm`

Form/confirm input intercepted before normal mode at `:124-131`
(`handleDeleteConfirmInput` then `handleCustomModelFormInput`).

### Views
- `CustomModelFormView` (presentational): `src/tui/components/CustomModelFormView.tsx`
  (props `:18-29`; rows `:74-110`; footer `:113-117`)
- Fields: `src/tui/constants/customModelForm.ts:38-96` —
  `id`(text,**required**), `displayName`(text), `facade`(select:
  openai/codex/claude/gemini/zai), `contextWindow`(number), `maxOutputTokens`(number),
  `compactionThreshold`(text), `reasoning`(bool), `hasVision`(bool)
- `DeleteCustomModelConfirmView`: `src/tui/components/DeleteCustomModelConfirmView.tsx`
  (props `:11-18`; `y/Enter` confirm, `n/Esc` cancel)

### Input handlers — `src/tui/inputHandlers/customModelFormHandler.ts`
- `handleDeleteConfirmInput` `:20-40`; `handleCustomModelFormInput` `:47-165`
  (Esc cancel; Enter save; ↑↓ field nav; ←→ cycle select/toggle bool; text entry)

### State + validation + submit — `src/tui/hooks/useCustomModelFormState.ts`
- `saveCustomModelForm` `:96-143` (requires `values.id` `:105-107`)
- `deleteCustomModelConfirmed` `:149-162`
- mode union: `src/tui/types/customModelMode.ts:13-41`

### Persistence — `src/tui/services/customModelCrudService.ts`
- `saveCustomModel` `:30-68` (read-modify-write `getProfile`/`saveProfile`)
- `deleteCustomModel` `:77-107`
- Storage: `fspec-config.json` → `providers.openai.profiles.<name>.customModels[]`.
  Pure-TS `provider-config` util — **no NAPI call**.

### `[C]` badge & `customModelIds`
`customModelIds: Set<string>` per section is the source of truth for editable/
deletable models AND the `[C]` badge. Projected to `customModelIdsBySection`
(`ModelSelectorScreen.tsx:216-225`).

---

## PART 2 — Rust current state

- **Keybinds: NO a/e/d** — `views/model_selector/mod.rs` `handle_key` (`:225-289`)
  handles only Esc, /, r/R, ↑↓, Home/End, ←→, Enter; a/e/d fall to
  `_ => ModelSelectorEvent::Consumed` (`:287`). No form-state, no mode enum, no
  overlay rendering (`render` `:309-332` paints the browse list only).
- **`[C]` badge: RENDERS** — `rows.rs` `build_badges` `:104-124` (emits `[C]`
  first `:106-108`, yellow `:135`); legend `[C] Custom` `:26-27`; backing field
  `ModelEntry.is_custom` `rpc-types/src/lib.rs:347`.
- **Backend custom-model CRUD: DOES NOT EXIST** (blocking):
  - `codelet/sessions/src/profile_sections.rs` read-only; `CustomModelDef`
    `:87-89` has only `id` and derives `Deserialize` only (no `Serialize`);
    `load_local_server_profiles` `:105-134` reads only; no write/save fn (only a
    TS comment at `:104`).
  - `codelet/providers/src/custom/management.rs` is about custom *providers*
    (Rhai-scripted), NOT custom *models on profiles*; `resolve_custom_model_id`
    `:475` resolves aliases only.
  - RPC trait `codelet/rpc/src/lib.rs:168` + all transports
    (`transport/{mod,embedded,websocket}.rs`) expose only `list_providers()`.
    No `add/update/delete_custom_model`.
  - NAPI (`napi/src/session_bindings.rs`, `models/napi_bindings.rs`): no
    custom-model write bindings.
  - Action enum (`components/mod.rs`): no `AddCustomModel`/`EditCustomModel`/
    `DeleteCustomModel` variants.

---

## PART 3 — Scope of work for Rust parity

**⚠️ Backend write surface MUST be built first (blocking dependency):**

1. **Backend persistence (NEW)** — in `codelet/sessions/src/`: a config writer
   that read-modify-writes `fspec-config.json`
   `providers.openai.profiles.<name>.customModels[]` (mirror TS
   `saveCustomModel`/`deleteCustomModel`). Extend `CustomModelDef`
   (`profile_sections.rs:87`) to a full definition (`displayName, facade,
   contextWindow, maxOutputTokens, compactionThreshold, reasoning, hasVision`)
   **with `Serialize`**.
2. **Types/RPC surface (NEW)** — add `CustomModelDefinition` wire type to
   `rpc-types/src/lib.rs`; add `add/update/delete_custom_model` to the RPC trait
   (`rpc/src/lib.rs:168`), `SessionManagerHandle` (`handle_impl.rs:930`), all
   three transports, NAPI bindings, and `Action` variants (`components/mod.rs`).
3. **Form + confirm overlay views (NEW)** — port `CustomModelFormView` and
   `DeleteCustomModelConfirmView` as Rust overlays under
   `views/model_selector/`, plus the field table and a form-state/mode struct.
4. **Keybinds + input routing (`mod.rs`)** — add `Char('a'/'e'/'d')` arms in
   `handle_key` (`:235-288`), gated on profile sections + custom-model membership;
   add form/confirm input sub-handlers; route to the new Actions. The row
   projection needs to carry which model ids are custom (today only the boolean
   `is_custom` per `ModelEntry` exists — sufficient for the guard, but the
   edit path needs the full definition fetched back from config).

**Recommendation:** split into at least two cards — (1) backend custom-model
CRUD surface (persistence + RPC + NAPI + types), (2) UI keybinds/views/form. This
card (RPC-344) tracks the UI half and is BLOCKED until the backend half exists.
Given the size, consider re-estimating once the backend card is scoped.
