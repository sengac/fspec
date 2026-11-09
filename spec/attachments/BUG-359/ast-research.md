# BUG-359 AST research
- `grep -rn current_frame_glyph rust/fspec-tui/src` found the definition in components/spinner.rs:48 (`#[must_use]`) and real uses in components/loading_dialog.rs:80 and components/menu_bar/chips.rs. In views/agent/input_transition.rs it was only imported (line 12) and discarded (line 258).
- `paint_spinner_line(area, buf, frame_index, message, hint)` (components/spinner.rs) takes the index and paints `DOTS_FRAMES[frame_index]`.
