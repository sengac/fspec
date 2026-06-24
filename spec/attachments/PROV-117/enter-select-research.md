# PROV-117 (reopened) — Research: Enter on a model row does not actually SELECT the model

## Symptom (user report)
> "You made the expand/collapse work, but pressing Enter doesn't select the model."

After the first PROV-117 pass, Enter on a **collapsed provider header** now toggles expansion
(TS parity). But Enter on a **selectable model row** does not produce an observable selection:
the `/model` view does not appear to close + apply the chosen model.

## Why the existing PROV-117 test missed this
`codelet/fspec-tui/src/views/model_selector/tests_enter_expand.rs::enter_on_model_row_emits_selection`
builds a `loaded_view()`, calls `Home` to **force `has_selection = true`**, then asserts that
`handle_key(Enter)` returns `ModelSelectorEvent::Emit(Action::ModelSelected(..))`.

That is a **unit-level assertion on the emit contract only**. It proves the dispatch arm fires an
event when all guards are pre-satisfied. It does **not** exercise:
- the real `list_providers()` async data path,
- the natural cursor arrival on a model row (header-expand → Down) and whether `has_selection`
  is set on that path,
- whether the open path actually seeds a `session_id`,
- whether the **emitted payload resolves** to a real model downstream (apply + chrome repaint + close).

This is the same gap PROV-104 hit: unit tests pre-inject rows, so they never reproduce the
live-binary symptom. PROV-104 closed it with an `@microsoft/tui-test` e2e
(`e2e/prov-104-model-nav.test.ts`). PROV-117's select path needs the same net.

## Code trace (Rust)
1. `views/model_selector/dispatch.rs:105-140` — `KeyCode::Enter` arm:
   - copy `(selectable, provider_key, model_id)` from `rows[selected_index]`
   - if `!selectable` → `toggle_expansion` (header path — PROV-117 pass 1)
   - **GUARD (Rust-only)** `if !self.has_selection { return Consumed; }`  (PROV-101)
   - **GUARD (Rust-only)** `let Some(session_id) = self.session_id else { return Consumed; }`
   - else `Emit(Action::ModelSelected(session_id, provider_key, model_id))`
2. `rows.rs:102` — model row carries `model_id: model.id.clone()` (the **RAW** id, including any
   `-YYYYMMDD` date suffix).
3. `app/dispatch_model_thinking_dialogs.rs:46-73` — `handle_model_selected`:
   - caches RAW id via `agent_view_store.set_selected_model_id(...)` (RPC-337 `(current)` marker)
   - spawns `backend.set_session_model(session_id, provider_id, model_id)` with the RAW id
   - re-fetches `get_model_info` for chrome repaint
4. `views/navigator.rs:122-126` — `Action::ModelSelected(..)` while view==ModelSelector flips back
   to `ViewMode::Agent` (this is what closes the selector).
5. `app/dispatch_model_selector.rs:21-30` — open path seeds `session_id` + `current_model` from
   `agent_view_store`.

## TS reference (the contract we must match)
- `src/tui/components/ModelSelectorScreen.tsx:203-210` — Enter on `model` → `selectModel(...)`
  → `onSelectModel(sel)` → `onClose()`. **No `has_selection` / `session_id` guards.**
- `src/tui/services/modelInitializationService.ts:42-101` — `createModelSelection` sends
  `modelId: extractModelIdForRegistry(model.id)` (date suffix `-YYYYMMDD` STRIPPED) and keeps
  `apiModelId: model.id` separately. Persistence/lookup use the **registry-normalized** id on
  **both** sides (`AgentView-model-persistence.test.tsx:356-357`).

## Candidate root causes (worker must reproduce, then confirm precisely)
1. **Payload-shape mismatch (strongest for real cloud models).** Rust emits the RAW `model.id`
   (with `-YYYYMMDD`), but the registry-backed apply path expects the normalized id
   (`extractModelIdForRegistry`). The selection is emitted and the view may close, but
   `set_session_model` resolves to nothing → "didn't select". The `(current)` marker also won't
   relight on reopen because the cached id mismatches. NOTE: this does NOT reproduce with
   suffix-free custom-model fixtures — the e2e fixture must include a model id WITH a date suffix
   to exercise it.
2. **`has_selection == false` silent no-op (Rust-only guard).** If the natural arrival on a model
   row (header Enter-expand re-anchors cursor on the header, then Down) does not set
   `has_selection`, Enter is a consumed no-op where TS would select. Verify `move_down` sets it.
3. **`session_id == None` silent no-op (Rust-only guard).** If the open path ran with no current
   session, every Enter on a model row is a no-op. TS does not gate on session here.

## Recommended verification (deterministic, network-free)
Follow the `e2e/prov-104-model-nav.test.ts` harness exactly:
- temp `$HOME/.fspec/fspec-config.json` with an `openai` local-server profile carrying
  `customModels` (surface as SELECTABLE rows offline, MODEL-004).
- **Add at least one custom model whose id carries a `-YYYYMMDD` suffix** to exercise root cause #1.
- Drive: open Work Agent → `/model` → filter/expand → Down onto a model → **Enter**.
- Assert end-to-end selection: the `/model` view **closes** (returns to agent view) AND the session
  header / `(current)` marker reflects the chosen model — not merely that an event was emitted.

## Files in scope
- `codelet/fspec-tui/src/views/model_selector/dispatch.rs` (Enter arm; supervisor-shared? NO — view-local)
- `codelet/fspec-tui/src/views/model_selector/rows.rs` (model_id payload — normalize like TS)
- `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs` (handle_model_selected / apply)
- `e2e/prov-117-model-select.test.ts` (NEW — e2e regression net)
- `spec/features/model-selector-enter-key-behavior.feature` (add e2e select scenario)
