# PROV-104 — Re-opened: e2e gap & "arrow keys do nothing" root cause

## Why this card was re-opened

PROV-104 was marked `done` with six scenarios in
`spec/features/model-view-scroll-viewport.feature`, all backed by **Rust unit
tests** (`codelet/fspec-tui/src/views/model_selector/tests_scroll.rs`,
`scroll_tests.rs`). Every one of those tests constructs a `ModelSelectorView`
with **rows already populated** (via a test helper that injects providers
synchronously) and then asserts paint/scroll geometry.

User report against a **freshly built binary**: opening `/model` and pressing
Up/Down does **nothing at all** — the cursor never moves and the list never
scrolls.

The unit tests cannot catch this because they bypass the real data path. There
was **no end-to-end (tui-test) verification** that the live binary:
1. actually populates model rows in the `/model` view, and
2. moves the highlighted row in response to Up/Down.

This is the ACDD gap: acceptance criteria were validated against a stubbed
in-memory view, not against real data flowing through the real binary.

## Verified event/data path (DeepSearch + direct reads)

Keyboard routing is **fully wired** — this is NOT a swallowed-key bug:

- `app/events.rs::App::run` reads crossterm events → `App::handle_event`
- `App::handle_event` stages: DisconnectDialog → Compositor → Navigator → app fallback
- `Navigator::handle_event` (navigator.rs:89-97) matches
  `ViewMode::ModelSelector => self.handle_model_selector_event(event)`
- `navigator_events.rs:57-87` → `model_selector.handle_key(key)`
- `model_selector/dispatch.rs:69-76` → `KeyCode::Up => move_up()`, `Down => move_down()`
- `navigation.rs:16-49` → moves `selected_index`, calls `adjust_scroll()`

Nothing upstream consumes bare Up/Down before the model selector.

## Actual root cause: rows are empty / header-only when keys arrive

1. Opening `/model` (`dispatch_model_selector.rs:21-31`,
   `handle_open_model_selector_view`) seeds session + current model, then
   **spawns an async `backend.list_providers()`** (lines 46-58). Rows are only
   built later when `Action::ListProvidersLoaded` arrives →
   `set_providers` → `rebuild_rows` (state.rs:44-78, 124-126).

2. Until that async result lands, `self.rows` is empty and
   `has_selection == false`. `move_up`/`move_down` take the `!has_selection`
   branch → `anchor_first_selectable()`; on empty rows `first_selectable`
   returns 0 and there is no selectable row → visually nothing happens.
   Subsequent presses call `move_*_skipping_headers`, which early-return `None`
   on empty/header-only rows → `selected_index` never changes.

3. Even after `list_providers()` returns, **built-in providers carry no models
   unless credentials are configured** (`sessions/src/handle_impl.rs:903-991`):
   cloud model rows are credential-gated via
   `cloud_models::provider_has_credentials` + the models.dev catalog
   (`build_cloud_registry`, network/cache gated). With no creds and a cold
   offline cache, the projection is **provider headers only** — every row is
   non-selectable, so header-skipping navigation has nothing to land on →
   "nothing moves."

4. Errors from `list_providers()` are **silently swallowed**
   (`if let Ok(providers)` at dispatch_model_selector.rs:53) — a failed load
   leaves the view permanently empty with no user-visible signal.

## What "done properly" requires (to be turned into scenarios)

- An **e2e tui-test** that launches the real `fspec` binary, opens `/model`,
  waits for real model rows to render, presses Down then Up, and asserts the
  highlighted/selected row actually changes (and the viewport follows on a
  list taller than the window).
- The e2e test must control the data so rows are deterministically populated
  (candidate approaches below — needs a decision).
- Define expected behaviour when rows are header-only / empty (loading state,
  empty state, error state) so the test asserts something other than silent
  inertia.

## Open question — how does the e2e test get real, deterministic model rows?

Candidate approaches (mirror existing e2e patterns):
- **A. `--features test-stub-provider`** (like rpc-072): registers `stub/canned`
  and pins the default model. Need to confirm the stub provider surfaces a
  selectable model row in `list_providers()` (it pins a default model but may
  not appear as a catalog row).
- **B. Configure a local-server profile / custom model** in a temp
  `~/.fspec/fspec-config.json` so `build_local_profile_sections()` yields
  selectable rows independent of network/credentials.
- **C. Set a provider API key env var** (e.g. `ANTHROPIC_API_KEY`) so
  credential-gated cloud rows populate — but this depends on the models.dev
  catalog being reachable/cached, so it is the least deterministic.

Decision needed before writing the test (see Example Mapping questions).

---

## DEFINITIVE ROOT CAUSE (found by running the e2e test with real data)

Running `e2e/prov-104-model-nav.test.ts` against the freshly built binary
rendered the live `/model` view with a real provider tree:

```
Select Model (64 models)
 ▶ OpenAI API (0 models)
 ▶ Anthropic (18 models)
 ▶ Cohere (0 models)
 ... (all providers COLLAPSED) ...
 ▶ 📁  openai: prov104-local (20 models)
```

Two facts combine into the "Up/Down does nothing at all" symptom:

1. **Collapse-by-default (RPC-342).** On open, every provider section is
   COLLAPSED; only the section containing the *current* model is auto-expanded.
   On a fresh install there is no current model (the board even shows
   "Session creation declined: no default model is set"), so **nothing is
   expanded** → the row projection is **provider headers only**, with **zero
   selectable rows**.

2. **Header-skipping navigation diverges from the TS reference.** The Rust
   helpers `move_up_skipping_headers` / `move_down_skipping_headers`
   (components/model_selector_dialog_rows.rs:57-90) only ever land on
   `selectable` (model) rows and **wrap around** with modulo. With no
   selectable rows they return `None`, so `selected_index` never changes —
   the cursor is frozen on row 0 and the user can only ever expand whatever
   header happens to sit at index 0.

   The **TypeScript reference does NOT skip headers**. `useModelSelectorState.ts`
   `navigateDown`/`navigateUp` (lines 218-246) move through `filteredFlatItems`
   by `Math.min/Math.max(currentIdx ± 1, …)` — a **clamp over the full flat
   list, headers included, no wrap**. So in TS the cursor moves down onto a
   collapsed provider header, and the user presses Right/Enter to expand it.
   The Rust port broke this contract.

### The fix (TS parity, `@ts-parity`)
Replace the header-skipping + wrap navigation with a clamped move over the
full row list (headers included), matching `navigateDown`/`navigateUp`:
- Up/Down move to `selected_index ± 1`, clamped to `[0, rows.len()-1]`.
- The cursor may rest on a header; Right/Enter expands it (Enter on a header
  is already a consumed no-op per PROV-101).
- Keep `adjust_scroll()` after each move (RPC-340/PROV-104 scroll-follow).
- Re-verify End/PageUp/PageDown and the "nothing selected" PROV-101 seed
  against the clamped model.

This is why unit tests passed: every PROV-104 unit test injects rows that
already contain expanded, selectable models, so the header-skipping helper
always had somewhere to land. Only the collapse-by-default fresh-open path
(real data) exposes the frozen cursor.

### Secondary findings
- A failed/empty `list_providers()` is silently swallowed
  (dispatch_model_selector.rs:53 `if let Ok`).
- There is no distinct *loading* state: before rows arrive the view shows the
  same `"No providers available"` placeholder used for the empty case.
- Header selection has **no text marker** ("▸" is painted only on selectable
  model rows), so header-cursor position is style-only (REVERSED) and not
  visible in a plain PTY text buffer — relevant to how the e2e asserts.
