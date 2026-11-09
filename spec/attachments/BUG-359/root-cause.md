# BUG-359 root cause

`paint_spinner` in rust/fspec-tui/src/views/agent/input_transition.rs does three things:
1. Computes the frame index `(elapsed_ms / DOTS_INTERVAL_MS) % DOTS_FRAMES.len()`.
2. Paints the row with `paint_spinner_line(area, buf, idx, ..)`.
3. Evaluates `let _ = current_frame_glyph(elapsed_ms)`.

Step 3 is a `#[must_use]` call whose glyph is discarded, so it is dead code. The painted glyph comes only from the index passed to `paint_spinner_line`. Nothing reads the extra call, so the spinner row does not change. The call is a leftover that predates the ff53a2e3 directory rename (no later commit touched it).

Fix: remove the call and the now-unused `current_frame_glyph` import. A source-shape test pins that paint_spinner paints through paint_spinner_line and computes no discarded glyph.

## Red / green (2026-11-02)
- Red (run 4): `paint_spinner_does_not_compute_and_discard_a_spinner_glyph` failed at :46 on the `current_frame_glyph(` assertion.
- Green (run 5): bug359 1/1 and multilineinput_parity_rpc095 12/12 (regression). Clippy (`--lib` + test, `-D warnings`) is clean.
