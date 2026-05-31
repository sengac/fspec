# RPC-097 — Rust Port Bug Analysis

## Root Cause: Flag Set But Never Read by Render Pipeline

The Rust port's `handle_session_cycle` (RPC-096) resolves `NavTarget::CreateDialog`
correctly but the side-effect it performs **never causes the dialog to actually
appear** on screen.

**File:** `codelet/fspec-tui/src/app/dispatch_rpc024.rs` lines 37–55

```rust
match target {
    NavTarget::Session(idx) => self.switch_to_session_index(idx),
    NavTarget::CreateDialog => {
        // RPC-096: preserve the typed draft (the user has not
        // left the current session — they are just summoning a
        // modal).
        self.agent_view_store.request_create_session_dialog_no_auto();
    }
    NavTarget::Board => { /* ... */ }
}
```

`request_create_session_dialog_no_auto` only flips a store field:

**File:** `codelet/fspec-tui/src/store/agent_view.rs` lines 241–244

```rust
pub fn request_create_session_dialog_no_auto(&mut self) {
    self.show_create_session_dialog = true;
    // … no compositor side-effect, no Action emit …
}
```

Nothing in the App's render path consumes `show_create_session_dialog`. The
**only** code path that ever pushes the `CreateSessionDialog` component onto
the `Compositor` is `handle_open_create_session_dialog` in
`dispatch_rpc060.rs`:

**File:** `codelet/fspec-tui/src/app/dispatch_rpc060.rs` lines 38–57

```rust
pub(crate) fn handle_open_create_session_dialog(
    &mut self,
    preselect: Option<CreateSessionOption>,
) {
    if self.compositor.contains(CREATE_SESSION_DIALOG_ID) {
        return;
    }
    let work_unit_context = self
        .agent_view_store
        .current_session()
        .cloned()
        .and_then(|sid| {
            self.agent_view_store
                .work_unit_context_for(&sid)
                .cloned()
        });
    let dialog = CreateSessionDialog::new(preselect, work_unit_context)
        .with_action_tx(self.action_tx.clone());
    self.compositor.push(Box::new(dialog));
}
```

That helper is only invoked from `try_dispatch_rpc060` when
`Action::OpenCreateSessionDialog { preselect }` arrives — there is **no
listener that translates `show_create_session_dialog = true` into a compositor
push**.

### Net effect

Shift+Right at the end of the session list:

1. `AgentView::handle_event` → emits `Action::SessionNext`.
2. `App::dispatch` → `handle_session_cycle(1)`.
3. `agent_view_store.navigate_next()` → `NavTarget::CreateDialog` ✓
4. `request_create_session_dialog_no_auto()` → flag set; ⚠️ dialog **NOT** pushed.
5. Frame renders unchanged. User sees nothing.

---

## Visual Drift From TS Ink Source

Even when the dialog *is* mounted (via `Action::OpenCreateSessionDialog` from
the `/isolation` slash pick, RPC-060), it does not match the TS source.

**File:** `codelet/fspec-tui/src/components/create_session_dialog.rs` lines 209–257

| Aspect              | TS Ink (canonical)                              | Rust port (current)                |
|---------------------|--------------------------------------------------|--------------------------------------|
| Selected option     | `backgroundColor='blue', color='white', bold`   | Prefix marker `▸` + raw spans       |
| Unselected option   | `color='gray'`                                  | Prefix marker `○` + raw spans       |
| Button padding      | `' Yes '` (1 space each side)                    | Bare text + 2-space outer gap        |
| Footer separator    | `|` (ASCII pipe U+007C)                          | `│` (U+2502 box-drawing)             |
| Footer dim          | `<Text dimColor>` (all-grey)                     | dialog_theme footer styling          |
| Centering           | `<Box justifyContent="center">`                  | row-level alignment via render_dialog |
| Border colour       | `cyan`                                            | `Accent::Cyan` ✓ (already matches)    |

The user requirement is: **"You must EXACTLY copy the existing typescript
dialogs and options."** — the divergence in selection styling and footer
glyph must be reconciled.

---

## Fix Direction (Implementation Sketch)

### Defect A: wire Shift+Right to actually push the dialog

`dispatch_rpc024.rs` `NavTarget::CreateDialog` branch must emit
`Action::OpenCreateSessionDialog { preselect: None }` (or directly call
`self.handle_open_create_session_dialog(None)`).

The store flag `show_create_session_dialog` either:

* gets repurposed as a render-pipeline subscription point (a system that
  watches the flag and pushes the dialog), OR
* is removed/deprecated in favour of the action-driven path that already
  works for `/isolation` (RPC-060).

Recommended: action-driven path (simpler, already proven, no fan-out).

### Defect B: visual parity

Refactor `CreateSessionDialog::render` to:

1. Build one DialogRow per visual line:
   * Row 1: title (`"Start New Agent?"` or `"Work on <id>?"`) — bold.
   * Row 2: description — dim.
   * Row 3: blank.
   * Row 4: three buttons centered, each formatted as ` Yes ` /
     ` Yes - Isolated ` / ` Cancel `. The selected one carries a span with
     `bg=Blue, fg=White, bold`. The unselected ones carry `fg=Gray`.
   * Row 5: blank.
   * Row 6: footer `"← → Select | Enter Confirm | Esc Cancel"` (ASCII pipe,
     dim).
2. Continue to delegate the outer border + sizing to
   `dialog_theme::render_dialog` (the base dialog code).
3. Remove the `▸`/`○` markers (no analogue in TS source).

### Reusing the Base Dialog

Per user instruction "re-use the base dialog code in rust to create the
dialogs for confirmation":

* `dialog_theme::render_dialog` + `FspecDialog` + `DialogRow` is the canonical
  Rust base dialog primitive (paints the rounded border, title, centered rows,
  footer separator, min-width, Accent colour).
* The fixed `CreateSessionDialog` continues to delegate to it (no hand-rendered
  `Block::default()` or `Paragraph` allowed).
* All visual differences against the TS source are expressed by constructing
  `DialogRow`s with the right `Span` styling, not by bypassing the base
  dialog.
