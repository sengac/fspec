# 11 — Open Questions and Risks

This document lists decisions to resolve before Example Mapping starts on
RPC-002 and the technical risks the port carries. The order of work-unit
breakdown depends on these answers.

---

## Open questions

### Q1. Alt-screen vs inline rendering mode?

- **Codex** runs ratatui in **inline mode** (no alt-screen) and writes
  scrollback to the host terminal via DECSTBM. This works because Codex
  is a one-stream chat REPL.
- **fspec** is a multi-pane Kanban / agent / board / file-search app.
  We almost certainly want **alt-screen mode** (full-screen TUI) for the
  primary workflow. This gives us a stable bounded canvas, simpler input
  handling, and parity with the Ink experience.
- **Recommendation:** **alt-screen**. Add an inline-mode subset later if
  we want a "minimal" composer-only invocation (similar to how Codex
  handles single-shot prompts).
- **Impact on breakdown:** alt-screen is the default for ratatui and
  needs no extra crate. Confirm and move on.

### Q2. Adopt `rat-event` / `rat-salsa`, or roll our own Compositor?

- `rat-event` provides the `Dialog` qualifier (literal 1:1 to our
  `InputPriority::CRITICAL`) and a documented event-routing trait
  family.
- A homegrown Helix-style Compositor is ~30 LoC.
- **Trade-off:** rat-salsa = less DIY, but adds a major dep family
  (rat-event, rat-widget, rat-focus, rat-popup, rat-dialog, rat-text,
  rat-window, rat-theme). Substantial learning curve. Custom code is
  trivial.
- **Recommendation:** **roll our own.** 30 LoC is cheaper than learning
  a framework, and the contract maps exactly to our Ink design.
- **Impact:** affects work-unit 02 (Input Compositor).

### Q3. Adopt `tui-realm` for React-like ergonomics?

- `tui-realm` is the closest React-mental-model framework on ratatui.
- BUT its focus-based event routing fights our priority-based dispatch.
- **Recommendation:** **no**. Stay close to bare ratatui.

### Q4. Implement `MultiLineInput` on `tui-textarea`?

- `tui-textarea` covers rope buffer, undo, paste, word-wrap, search,
  cursor styling.
- Custom code on top: history, slash palette, file mention, submit
  semantics (~340 LoC).
- **Alternative:** rebuild from scratch. ~700 LoC. More control, but
  re-implements undo / paste / wrap.
- **Recommendation:** **use `tui-textarea`.**

### Q5. Use `tui-popup` for Dialog rendering, or hand-roll centered-rect?

- `tui-popup` is in the official `ratatui-org/tui-widgets` mono-repo
  (long-term viability signal). Supports drag-to-move (we don't use it
  today but might).
- Hand-rolled `centered_rect` + `Clear` + `Block` is ~10 LoC.
- **Recommendation:** **hand-roll first**, swap to `tui-popup` only if
  drag-to-move becomes desired. Keeps deps minimal.

### Q6. Use `tui-widget-list` as the VirtualList base, or roll our own?

- `tui-widget-list` covers virtualization, variable heights, basic kbd
  nav, wrap-around.
- Doesn't cover: group selection, scroll vs item modes, scrollToEnd,
  mouse-wheel velocity, native text-selection toggle, lazy-mode
  `getItems(start, end)`.
- **Recommendation:** **try `tui-widget-list` first.** If group
  selection or lazy mode don't compose cleanly, fork to a custom
  implementation. Either way, the surface area we control is small (a
  thin wrapper).

### Q7. Lazy mode for VirtualList - same shape, or rethink?

The Ink lazy mode is `(itemCount, getItems(start, end))`. In Rust we'd
either:

- (a) Mirror it: `Box<dyn Fn(usize, usize) -> Vec<T>>`.
- (b) Use `tui-widget-list`'s `ListBuilder<T>` (per-index queries).
- (c) Define a `ListItems<T>` trait with both `len()` and a range
  accessor.

**Recommendation:** option (c). Cleanest abstraction, decouples from any
specific crate.

### Q8. Group selection - keep the API or simplify?

The Ink VirtualList has both `groupBy` (item -> id) and `groupByIndex`
(index -> id) for lazy mode. In Rust we can collapse to one trait:

```rust
pub trait Grouped {
    fn group_id(&self, index: usize) -> Option<GroupId>;
}
```

implemented either by closures or by the consumer.

**Recommendation:** unify. Less surface area.

### Q9. How does the ratatui frontend talk to fspec?

Per `rpc-002-feasibility.md`, via tarpc with two transports:

- **Embedded** (in-process, ratatui linked into the same binary as the
  fspec engine).
- **WebSocket** (ratatui as separate binary, connecting to a fspec
  daemon).

**Question:** does the ratatui crate itself need to expose any UI-RPC,
or does it only **consume** RPCs? Probably the latter, but worth
confirming before the trait surface is defined.

**Recommendation:** define `trait FspecBackend` based on the Ink TUI's
current `useApi` calls; implement two backends; ratatui consumes the
trait.

### Q10. Will the existing Ink TUI continue to be maintained during migration?

If so, we need a feature-parity matrix tracked over time. If not (Ink
gets frozen at the start of the port), we just need a finish-line check.

**Recommendation:** **freeze Ink at port start.** Otherwise we'll be
chasing a moving target and the port will balloon.

---

## Risks

### R1. Group selection preservation across mutations is unique to fspec

No prior art in any surveyed project. We're rolling this from scratch.
Risk: subtle bugs in lazy mode where we don't have items to inspect.
Mitigation: extensive unit tests; index-based group_id function; clearly
documented invariants.

### R2. Mouse-wheel velocity acceleration timing on Linux vs macOS vs Windows

The 150 ms / cap-at-5 numbers were tuned in the Ink version on macOS.
crossterm delivers events differently on different platforms.

Mitigation: keep the constants tunable in a config struct; test on all
three OSes during the port slice that lands `VirtualList`.

### R3. Native text-selection toggle interactions

Disabling mouse capture mid-session can lose events on some terminals.
The 5-second debounce is empirically tuned.

Mitigation: keep the timer configurable; add a "disable text selection
mode" config to fall back to always-on mouse capture.

### R4. ratatui constraint layout vs Yoga - some Ink layouts don't
have direct constraint equivalents

Specifically, layouts that use `flexShrink: 1, flexGrow: 1, minWidth:
N` rules might need to be redesigned. Most fspec layouts are simple
enough to not hit this, but `UnifiedBoardLayout` has nested complex
splits that need verification.

Mitigation: prototype `UnifiedBoardLayout` early to flush out any
layout-translation gaps.

### R5. tui-widget-list might not support our group-selection model

If `ListableWidget` insists on owning the `selected` state, we'll have
to shadow it. This may be ugly.

Mitigation: build a 1-day spike to confirm `tui-widget-list` accepts
external selection state. If not, fall back to rolling our own.

### R6. tarpc embedded transport with shared async runtime

The ratatui frontend needs its own tokio runtime; the fspec engine has
one too. In embedded mode, do we share? The feasibility doc covers this,
but if shared, the ratatui crate must accept a `Handle<tokio::Runtime>`
rather than spawning its own.

Mitigation: design the frontend crate to accept a runtime handle from
the host; only spawn its own in standalone mode.

### R7. Insta snapshot tests will be terminal-width-dependent

Snapshots must be taken at a fixed terminal size or they'll be flaky.

Mitigation: standardise on `TestBackend::new(120, 40)` for snapshots;
document this in the testing standards doc.

### R8. Performance regression risk

Ink rendering via the React reconciler is slow. ratatui's immediate
mode is fast - but only if we avoid allocation hotspots in the render
path. The Ink VirtualList uses a `scrollbarCache` exactly because the
React tree was slow.

Mitigation: budget render at < 1 ms per frame. Use `cargo flamegraph`
on the AgentView (worst case - streaming chat with 500+ items) early.
Cache rendered `Vec<Line>` per item only if profiling demands it.

### R9. Underscore: Ink's "soft-fallback to plain useInput" pattern is
gone

Currently, `AgentSelector` and `ConfirmPrompt` work in standalone
contexts (during `fspec init`) by detecting no `<InputProvider>` is
present and falling back to plain Ink `useInput`. In the Rust port,
every binary uses the Compositor, so there's no fallback to
"non-interactive Inquirer-like prompt".

This means: if we want `fspec init` to remain a non-TUI inquirer-like
flow, we need a separate code path that uses crates like `inquire` or
`dialoguer` and bypasses ratatui entirely. **This is a design question
for `fspec init` separately from the TUI port.**

Mitigation: out of scope of RPC-002. Tag a follow-up work unit for
"fspec init non-TUI fallback".

### R10. "I want to know if there are libraries that use ratatui as
the base that assist with this" - confirmed: yes, several.

But none cover the full set. The composition we recommend is:

```
ratatui core           → render primitives, Layout, Scrollbar
crossterm              → input + parsed mouse events
tui-widget-list        → virtualization base (try first)
tui-popup              → optional, for Dialog drag-to-move (not now)
tui-textarea           → MultiLineInput rope buffer
tui-input              → optional, single-line dialog fields
ratatui-explorer       → optional, for AttachmentDialog file tree
ratatui-image          → optional, future inline image previews
syntect (precompiled)  → optional, code highlighting in agent view
nucleo                 → fuzzy match for file mention popup
```

Plus ~1500 LoC of custom code (Compositor + VirtualList wrapper +
Dialog wrapper + MultiLineInput glue).

---

## Decisions matrix (to be answered before Example Mapping)

| ID | Question | Default recommendation | Decision needed by |
|---|---|---|---|
| Q1 | Alt-screen vs inline | Alt-screen | Foundation slice |
| Q2 | rat-event vs custom Compositor | Custom (~30 LoC) | Foundation slice |
| Q3 | tui-realm vs bare ratatui | Bare | Foundation slice |
| Q4 | tui-textarea for MLI | Yes | MLI slice |
| Q5 | tui-popup for Dialog | No (hand-roll) | Dialog slice |
| Q6 | tui-widget-list for VL | Try first | VirtualList slice |
| Q7 | Lazy-mode API shape | `trait ListItems` | VirtualList slice |
| Q8 | Group API unification | Unify to one trait | VirtualList slice |
| Q9 | tarpc embedded shared runtime | Share via `Handle` | RPC slice |
| Q10 | Freeze Ink during port? | Yes | RPC-002 epic-level |

---

## Out of scope of RPC-002

- `fspec init` non-TUI fallback (separate work unit).
- Full-color theme system / theme switcher (later).
- Keybinding customisation (later).
- Internationalisation (later).
- Tests for every single Ink component (port slices each carry their
  own tests; meta-coverage tracked separately).
