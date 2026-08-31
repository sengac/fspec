# AST Research — MUX-004 (mux configuration dialog + /mux slash-popup entry)

Research performed with the AST search tool (GraphSearch `ast_search`) before
moving MUX-004 from `specifying` to `testing`. Scope: every symbol the card
adds, renames, or rewires.

## 1. `app/mux_parser.rs` — `parse_mux_command` (lines 74–104, complexity 12)

```rust
pub fn parse_mux_command(line: &str) -> Result<MuxSubcommand, MuxError> {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("/mux") else {
        return Err(MuxError::UnknownSubcommand(trimmed.to_string()));
    };
    let args: Vec<&str> = rest.split_whitespace().collect();
    if args.is_empty() {
        return Ok(MuxSubcommand::Toggle);   // ← MUX-004: becomes MuxSubcommand::Config
    }
    match args[0] {
        "on" => Ok(MuxSubcommand::On),
        "off" => Ok(MuxSubcommand::Off),
        "h" | "horizontal" => Ok(MuxSubcommand::Orientation(MuxOrientation::Horizontal)),
        "v" | "vertical" => Ok(MuxSubcommand::Orientation(MuxOrientation::Vertical)),
        "save" => Ok(MuxSubcommand::Save),
        "default" => Ok(MuxSubcommand::Default),
        "help" => Ok(MuxSubcommand::Help),
        first => { /* pane count 2..=4, else pane list */ }
    }
}
```

`MuxSubcommand` variants: `Toggle` (renamed `Config` by MUX-004 R1), `On`,
`Off`, `Orientation(MuxOrientation)`, `PaneCount(usize)`, `PaneList { panes,
split_percent }`, `Save`, `Default`, `Help`.

## 2. `app/dispatch_mux.rs` — `apply_mux_subcommand` (lines 58–109, complexity 15)

The `Toggle | On | Off` group routes to `handle_mux_toggle()` /
`handle_mux_on()` / `handle_mux_off()`. The trailing block re-enters the Mux
view + `mux_sync_window()` + `recompute_rects()` whenever
`navigator.mux.config().enabled`. MUX-004 changes: `Config` opens the dialog
instead of toggling; the two new `Action` arms (`MuxConfigApplied`,
`MuxConfigAppliedAndSaved`) land in this file's dispatch surface and apply the
draft (orientation/panes/scale) plus the enabled flip (R7).

## 3. `views/multiplex/types.rs` — `MuxConfig` (lines 39–51)

```rust
pub struct MuxConfig {
    pub orientation: MuxOrientation,   // Horizontal (default) | Vertical
    pub splits: Vec<u16>,              // BUG-166: n-1 entries, last = remainder
    pub panes: Vec<MuxPaneKind>,       // Board | Agent | ChangedFiles | Checkpoints
    pub focused_pane: usize,
    pub enabled: bool,
}
```

`MultiplexLayout` (same file, line 54+) owns the live state: `config`,
`pane_rects`, `divider_rects`, drag fields, `focus`, `window_start`,
`sessions`, `rendered_panes`, `pending_new_agent`, `body_area`,
`pre_mux_view`. `Default for MuxConfig` (mod.rs lines 40–48) = default preset
shape with `enabled = false` (horizontal, Board | Agent, `[50]`, focus 1).

## 4. `app/state.rs` — `save_mux_config` (lines 396–401)

```rust
pub fn save_mux_config(&mut self) -> Result<(), String> {
    self.mux_state
        .config_mut()
        .clone_from(self.navigator.mux.config());
    self.mux_state.save()
}
```

The `MuxConfigAppliedAndSaved` arm must: apply the draft to
`navigator.mux` (orientation/panes/scale/enabled) FIRST, then call this
(persistence reads `navigator.mux.config()`). `mux_state().config()` is kept
in lockstep by the end of every `App::dispatch` (dispatch.rs, after
`navigator.apply_action`).

## 5. `views/agent/slash_commands.rs` — registry (lines 21–172)

`SlashCommandAction` enum (20 variants, ends with `Goal`, `Update`) — MUX-004
appends `Mux` + its `name()` arm (`"mux"`). `SLASH_COMMANDS` const slice adds
one row: `SlashCommand { action: SlashCommandAction::Mux, description:
"Configure the mux layout" }`. `filter_commands` picks the row up
automatically (prefix/substring/description tiers).

## 6. `app/dispatch_slash_commands.rs` — `handle_slash_command` (lines 27–158)

Per-variant `SlashCommandAction` routing. `Role` arm is the pattern for
`Mux`: `self.handle_open_role_dialog()` → direct call into a dedicated
`app/dispatch_*.rs` helper (stays under the 300-LoC ceiling). MUX-004 adds
`SlashCommandAction::Mux => self.handle_open_mux_config_dialog()`.

## 7. `app/dispatch_role_dialog.rs` — `handle_open_role_dialog` (lines 29–42)

The idempotent-dialog-open pattern MUX-004 mirrors:

```rust
pub(crate) fn handle_open_role_dialog(&mut self) {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else { return; };
    if self.compositor.contains(ROLE_DIALOG_ID) { return; }
    let seed = self.agent_view_store.role_for(&session_id).map(str::to_string);
    let dialog = RoleDialog::new(session_id, seed).with_action_tx(self.action_tx.clone());
    self.compositor.push(Box::new(dialog));
}
```

MUX-004 equivalent: `handle_open_mux_config_dialog()` in a NEW
`app/dispatch_mux_config.rs` — NO session guard (the dialog is app-level, not
session-level), seeds the draft from `self.navigator.mux.config().clone()`,
idempotent via `self.compositor.contains(MUX_CONFIG_DIALOG_ID)`.

## 8. `components/mod.rs` — `Action` enum

Last variant before the `}` (line ~1224): `MuxEnterWorkUnit(String)` in the
`// MUX-001` block. MUX-004 adds to the same block:
`MuxConfigApplied(MuxConfig)` and `MuxConfigAppliedAndSaved(MuxConfig)`.
`App::dispatch` routes them via an extended `is_mux_action` match (the
`a if App::is_mux_action(a)` arm at dispatch.rs line ~315 runs BEFORE
`navigator.apply_action`, and the mux-state lockstep + R6 auto-save at the
end of `dispatch` then keeps `mux_state` consistent — an exit-via-dialog
therefore auto-saves the `enabled=false` config for free).

## 9. `components/role_dialog.rs` / `thinking_level_dialog.rs` — dialog pattern

`Priority::Foreground`, stable id const, `with_action_tx(UnboundedSender<Action>)`,
`handle_event` returns `EventResult::Consumed(Some(callback))` where the
callback does `compositor.remove(&id)` on commit/close. `render` builds an
`FspecDialog { accent, title, rows, footer, min_width, query_row: None }` and
calls `dialog_theme::render_dialog`. Row builder:
`dialog_theme_rows::label_description_default_row(label, description,
selected, is_default)`.

## 10. `views/agent/dispatch_popups.rs` — `handle_popup_key` (lines 21–34)

Popup-pick path: `PopupOutcome::Selected(action)` → `self.slash_popup = None;
self.input.reset(); self.emit(Action::SlashCommandSelected(action));` —
"agent input is cleared" (R6) is satisfied by the existing `input.reset()`.

## 11. Test surfaces (integration-test seams)

- `App::new(Arc<dyn FspecBackend>)`, `app.dispatch(action)`,
  `app.handle_event(&Event)`, `app.render(area, buf)`,
  `app.compositor().contains(id) / layer_ids()`, `app.navigator().mux`,
  `app.mux_state().config()`, `app.set_mux_persist_dir(data, cwd)` (TempDir
  pattern from `tests/mux001.rs` save scenario), `app.try_recv_action()` +
  `app.next_pending_task()` (drain helper).
- `Navigator::new(theme, tx)` for component-free mux tests
  (`tests/mux001.rs` `fresh()` helper).
- `filter_commands("mux")` for the registry assertion without a live popup.
- `AgentView.slash_popup: Option<SlashCommandPopup>` and
  `AgentView.input: MultiLineInput` are `pub` fields — popup state is
  directly observable through `app.navigator().agent`.
- Compositor: `layer_ids()` (Vec<String>) + `contains(id)` for the
  "exactly one instance" assertions (R2 / example 5).

## 12. Affected existing tests (RED updates required by R1 supersession)

Bare `/mux` today = toggle. After MUX-004 it opens the dialog, so these
fixtures must switch to `/mux on` (or dialog commit):

- `rust/fspec-tui/tests/mux001.rs` — `bare_mux_and_on_off_and_help_parse`
  (asserts `Toggle`), `slash_mux_toggles_mux_mode_on_with_the_default_preset`
  (asserts view flip on bare submit), `app_with_mux` fixture (line ~1016),
  `slash_mux_off_returns_to_the_pre_mux_view` (line ~1087).
- `rust/fspec-tui/tests/bug164_mux_retained_on_session_exit.rs` —
  `app_with_sessions_and_mux` fixture.
- `rust/fspec-tui/tests/bug165_esc_exit_dialog_on_board_pane_in_mux.rs` —
  two `submit(app, "/mux")` call sites (lines 77, 207).
- `rust/fspec-tui/tests/bug166_mux_dividers_percentage_scale.rs` —
  `app.dispatch(Action::InputSubmitted("/mux"))` (line 535).
- `rust/fspec-tui/src/app/mux_parser.rs` inline test
  `bare_and_lifecycle_subcommands`.
- Inline `#[test]`s in `views/agent/slash_commands.rs` and the insta
  snapshot `slash_command_popup__centered_popup_80x24` (registry grew by one
  row — snapshot regeneration), plus `help_dialog_content_rpc397`
  expectations ("all 17 slash commands" wording) if they hard-code a count.
