# BUG-359 design
- Keep paint_spinner's index computation and its paint_spinner_line call.
- Delete `let _ = current_frame_glyph(elapsed_ms);` and drop `current_frame_glyph` from the `use super::spinner::{..}` import.
- Do not add a new index helper; the scope is the dead call only.
