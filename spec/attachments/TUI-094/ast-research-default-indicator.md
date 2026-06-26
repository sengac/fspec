# TUI-094 AST Research — (default) indicator parity

## Builder/constructor shape to extend

- `codelet/fspec-tui/src/components/dialog_theme_rows.rs:17`
  `pub fn label_description_row(label: &str, description: &str, selected: bool) -> DialogRow`
  — sole row builder; only caller in src is `thinking_level_dialog.rs:175`. Will add a
  sibling `label_description_default_row(label, description, selected, is_default)` to avoid
  regressing the existing signature (also covered by rpc027_dialog_theme.rs tests).

- `codelet/fspec-tui/src/components/thinking_level_dialog.rs:46`
  `pub fn new(session_id: SessionId, current_level: ThinkingLevel) -> Self`
  — keep stable; add `with_default_level(self, default: Option<ThinkingLevel>)` builder and a
  `default_index: Option<usize>` field computed from `LEVELS`.

## Callers / blast radius

- `label_description_row` callers (src): only `thinking_level_dialog.rs:175`. No ModelSelectorDialog
  src caller currently routes through it (grep confirms). Tests in rpc027_dialog_theme.rs assert its
  existing behavior — must stay unchanged.
- `ThinkingLevelDialog::new` callers: dispatch_model_thinking_dialogs.rs:36 (open path), plus tests
  rpc022/rpc027/in-file snapshot. Builder keeps all compiling.

## Data source (TUI-093, unchanged)

- `codelet/sessions/src/default_thinking_level_persistence.rs:192`
  `load_default_thinking_level_opt() -> Option<ThinkingLevel>` (and `_with_dirs` core).

## Wiring point

- `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:22` `handle_open_thinking_dialog`
  constructs dialog with only `current`; will chain `.with_default_level(load_default_thinking_level_opt())`.
