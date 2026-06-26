# RPC-353 — Mouse-wheel + Page/Home/End scroll for `/provider` and `/model`

**Author:** Supervisor (self-investigated via DeepSearch + direct reads)
**Scope:** Input routing + key/mouse handlers for the two full-screen mode-views.
**Goal:** Mouse wheel scrolls both lists (matching the chat view's velocity ramp), and
`/provider` gains PageUp/PageDown/Home/End (which `/model` already has).

> NOTE: This is an **enhancement beyond strict TS parity**. In TS, neither the
> ProviderSettings nor ModelSelector screens implement Page/Home/End or mouse wheel —
> that lives only in the `VirtualList` conversation component. The user explicitly wants
> these two screens to gain the same scroll affordances the chat view uses.

---

## 1. Root cause — Navigator drops all mouse events

- **File:** `codelet/fspec-tui/src/views/navigator_events.rs`
  - `handle_provider_settings_event` (`:24–27`):
    ```rust
    let Event::Key(key) = event else {
        return EventResult::ignored();
    };
    ```
    → **every `Event::Mouse` is discarded** before reaching the view.
  - `handle_model_selector_event` (`:57–60`): identical early-return.
- Routing entry: `navigator.rs:89–97` (`handle_event`) forwards the raw `Event` to these
  per-view translators, so the mouse event *arrives* at the navigator but is dropped here.

### Consequences
- **`/model`** already has a `handle_mouse` (`model_selector/dispatch.rs:188–199`):
  `ScrollUp → move_up()`, `ScrollDown → move_down()` — **but it is DEAD CODE**: a grep
  shows it's never called outside its own definition + tests. So the wheel does nothing.
- **`/provider`** has **no mouse handler at all**, and `list.rs:8` explicitly documents
  "no PgUp/PgDn/Home/End — RPC-157". So neither wheel nor paging keys work.

---

## 2. Canonical "chat view" scroll pattern to mirror

- **Routing:** `views/agent/dispatch.rs:150–158` — on `Event::Mouse(m)` it calls
  `handle_mode_view_mouse` / `handle_popup_mouse` / `handle_scrollback_mouse`.
- **Wheel handler:** `views/agent/mouse_dispatch.rs:78–102` (`handle_scrollback_mouse`):
  - Hit-tests the cached content rect.
  - `MouseEventKind::ScrollUp` → `self.scrollback_wheel.step(WheelDirection::Up)`,
    emits with the resulting velocity; `ScrollDown` symmetric.
- **Shared primitives (reuse these):** `components/scroll_viewport.rs`
  - `ensure_visible(scroll_offset, selected, visible_rows, total)` (`:46`).
  - `WheelDirection` enum (`:70`).
  - `WheelVelocity` accelerator (`:75+`) — 1×–5× ramp, caps at 5 when wheel events arrive
    faster than every 150 ms, resets to 1 after a 150 ms+ gap.
- **Clean in-view Page/Home/End key pattern:** `views/board.rs:132–148`.
- **`/model`'s existing paging methods to reuse:** `model_selector/dispatch.rs:77–96`
  (Home/PageDown/PageUp/End arms) → `navigation.rs:53–89` (`page_down`/`page_up` move one
  `visible_rows` worth across selectable rows).

---

## 3. Required behaviour (acceptance-criteria seeds)

### Mouse wheel routing (both views)
1. The Navigator forwards `Event::Mouse` to the active mode-view instead of dropping it:
   `handle_provider_settings_event` and `handle_model_selector_event` must route mouse
   events to a `handle_mouse` on their view.
2. `/model`'s existing `handle_mouse` (`dispatch.rs:188–199`) becomes **live** — wheel up
   scrolls toward the top, wheel down toward the bottom.
3. `/provider` gains a `handle_mouse`: `ScrollUp`/`ScrollDown` move the selection/window
   (mirroring `/model`'s wheel semantics), then `adjust_scroll()`.
4. Wheel scrolling uses the shared **`WheelVelocity` 1×–5× ramp** so fast scrolling moves
   multiple rows per event — same feel as the chat view.

### Keyboard paging (`/provider` — `/model` already has these)
5. `/provider` List mode binds **PageUp / PageDown** to move the selection by one
   `visible_rows` page (clamped, no wrap), then `adjust_scroll()`.
6. `/provider` List mode binds **Home** (first item) and **End** (last item), then
   `adjust_scroll()`.
7. Filter-mode (`/provider`) must NOT hijack these keys for text input — paging/Home/End
   only apply when `filter_mode` is false (printable-char accumulation unchanged).
8. `/model`'s existing Page/Home/End behaviour is preserved (regression-guard).

### Non-goals / unchanged
9. Arrow-key nav, Enter, Tab, filter, expand/collapse all unchanged for both views.
10. Mouse events outside the body/list rect (where applicable) should not be consumed
    spuriously.

---

## 4. Constraints / notes for implementer

- **DO NOT modify the chat/agent view** — it is the reference pattern.
- **Reuse `WheelVelocity` / `WheelDirection` / `ensure_visible`** from
  `components/scroll_viewport.rs`. Do NOT reimplement a wheel ramp.
- **`/model`:** wire its EXISTING `handle_mouse` through the navigator; optionally upgrade
  it to use `WheelVelocity` for multi-row steps. Keep arrow/page/Enter behaviour intact.
- **`/provider`:** add `handle_mouse` + Page/Home/End in `list.rs` (the live List-mode key
  handler is `handle_list_key`, `list.rs:27–`). Reuse `view.move_clamped(delta)` for the
  page step (it already clamps), and the existing `scroll_offset`/`adjust_scroll` plumbing.
- **EventResult contract:** consumed when the view handles the wheel/page; ignored
  otherwise so events still bubble correctly.
- **300-LoC ceiling** on every touched file. `navigator_events.rs` is ~124 lines;
  `list.rs` ~265; `model_selector/dispatch.rs` ~200 — if adding code pushes any over 300,
  extract a helper (e.g. a `provider_settings/mouse.rs` mirroring `agent/mouse_dispatch.rs`).
- **Tests:** add failing tests first (ACDD) that assert: (a) a `ScrollDown` event routed
  through the navigator moves the `/provider` and `/model` selection/scroll_offset;
  (b) PageDown/PageUp/Home/End move the `/provider` selection by a page / to ends;
  (c) the wheel velocity ramps under rapid events. Then implement to green.
- **Verify:** `cargo test -p codelet-fspec-tui --lib` green; no chat-view or `/model`
  snapshot regressions.

---

## 5. Verified line references (captured at investigation time)

```
Rust navigator.rs:89-97                       handle_event forwards raw Event to per-view translators
Rust navigator_events.rs:24-27                handle_provider_settings_event — drops non-Key (mouse) events
Rust navigator_events.rs:57-60                handle_model_selector_event — drops non-Key (mouse) events
Rust model_selector/dispatch.rs:188-199       handle_mouse (ScrollUp/Down → move) — DEAD CODE, never routed
Rust model_selector/dispatch.rs:77-96         Home/PageDown/PageUp/End key arms (already present)
Rust model_selector/navigation.rs:53-89       page_down / page_up (one visible_rows page)
Rust provider_settings/list.rs:8              "no PgUp/PgDn/Home/End — RPC-157"
Rust provider_settings/list.rs:27-            handle_list_key (live List-mode key handler)
Rust agent/dispatch.rs:150-158                Event::Mouse routing into handle_*_mouse
Rust agent/mouse_dispatch.rs:78-102           handle_scrollback_mouse — WheelVelocity.step pattern
Rust components/scroll_viewport.rs:46,70,75   ensure_visible / WheelDirection / WheelVelocity
Rust views/board.rs:132-148                   clean PageUp/PageDown/Home/End key pattern
TS  (both screens)                            NO page/home/end/wheel — this is an enhancement beyond TS parity
```
