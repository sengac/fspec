# RPC-093 (reopened) — Thinking Indicator Animation Parity Regression

**Status**: reopened 2026-05-30 — the original "Thinking accumulation in scrollback" landed
correctly, but the input-row Thinking indicator that drives the visual experience is
broken in three observable ways. The card is back in `specifying` so we can document the
correct parity and re-run the ACDD loop on the animation layer.

## 1. Symptoms reported (verbatim)

> "there's no animation like in the typescript version when it finishes ... also, the
> animation of the thinking seems like it's tied to the rendering of the scroll view,
> so it's not animating smoothly ... there's a cursor that shouldn't be in the
> animation" (see `Screenshot 2026-05-30 at 11.13.24 am.png`).

Three concrete regressions:

| ID | Symptom | Visible in screenshot |
|----|---------|-----------------------|
| **A** | Spinner cycles in lock-step with stream chunks instead of ticking at 80 ms | Frozen braille glyph mid-stream |
| **B** | Spinner→input swap is instantaneous; no finish animation | Atomic disappearance — n/a |
| **C** | Terminal cursor block visible *under* "Thinking…" | White inverse cell on the **T** of "Thinking" |

## 2. TypeScript source of truth

### 2.1 Spinner cadence — `src/tui/components/ThinkingIndicator.tsx`

```tsx
const SPINNERS = { dots: { frames: ['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏'], interval: 80 } };

useEffect(() => {
  if (!isActive) return;
  const timer = setInterval(
    () => setFrame(prev => (prev + 1) % frames.length),
    spinner.interval, // 80 ms
  );
  return () => clearInterval(timer);
}, [isActive, frames.length, spinner.interval]);
```

Even though Ink batches state updates at 60 fps (`INK_MAX_FPS = 60`,
`INK_FRAME_TIME_MS = 17`), the `setFrame` from the spinner timer **always triggers a
reconciliation** — Ink's render loop is not gated on external events.

### 2.2 Finish animation — `src/tui/components/InputTransition.tsx`

Phase machine: `'loading' | 'paused' | 'hiding' | 'showing' | 'complete'`.

```tsx
const CHAR_ANIMATION_INTERVAL_MS = INK_FRAME_TIME_MS;     // ≈17 ms
const CHARS_PER_FRAME             = 5;
const ANIMATION_PHASE_DELAY_MS    = 34;                   // 2 frames
```

Transitions:

1. `loading` → `<ThinkingIndicator>` (spinner ticks at 80 ms).
2. `isLoading` flips false → capture current visible text via `useThinkingText`
   into `capturedText`; enter `hiding`.
3. `hiding` — `setVisibleChars(prev => max(0, prev - 5))` every 17 ms; render
   `capturedText.slice(0, visibleChars)`. Total ≈ `ceil(28/5)*17 ≈ 102 ms` for the
   "Thinking... (Esc to stop)" string.
4. `34 ms` pause → enter `showing`, reset `visibleChars` to 0.
5. `showing` — `setVisibleChars(prev => min(placeholder.length, prev + 5))` every
   17 ms; render `placeholder.slice(0, visibleChars)`. Total ≈ `ceil(N/5)*17 ms`
   where N is the placeholder length.
6. `complete` — mount `<MultiLineInput />`.

If the user types a printable key during `hiding`/`showing`, an Ink
`useInputCompat(MEDIUM_PRIORITY)` handler **short-circuits** the phase to `complete`
and feeds the typed character into `MultiLineInput`.

If `isLoading` becomes true mid-`hiding`/`showing`, a `useEffect` resets the phase
to `loading` and the spinner resumes immediately.

### 2.3 Cursor visibility — `src/tui/components/MultiLineInput.tsx`

The inverse-space cursor block (`<Text inverse> </Text>`) is rendered **only inside
`MultiLineInput`**. Because `InputTransition` returns `<ThinkingIndicator>` during
`loading` and a plain `<Text dimColor>` during `hiding`/`showing`, the cursor block
is never reachable while the agent is busy or animating — it appears only when phase
reaches `complete`.

## 3. Rust regressions (diagnosis)

### 3.1 Regression A — spinner not smooth

**Root cause**: `codelet/fspec-tui/src/app/events.rs` only redraws when
`self.should_render == true`. `should_render` is set by `handle_event`,
`dispatch(Action)`, `Resize`, and `handle_paste` — **never by a periodic timer**.

The 16 ms `tokio::time::interval(RENDER_TICK)` *exists* but only consults
`should_render`:

```rust
_ = tick.tick() => {
    if self.should_render {        // ← gated; usually false between chunks
        guard.terminal().draw(|frame| { … })?;
        self.should_render = false;
    }
}
```

`InputTransitionState::Loading { elapsed_ms }` reads `spinner_started_at.elapsed()`
(real wall-clock time), so the *value* is always correct — but the screen is only
repainted when a chunk arrives, so the visible glyph jumps in bursts that align with
streaming output, not with the 80 ms spinner cadence. Exactly the "tied to scroll
rendering" effect the user reported.

RPC-095 rule [16] specced an `Action::SpinnerTick` emitted by a
`tokio::time::interval(80ms)` task — `grep` confirms it was never implemented.

### 3.2 Regression B — no finish animation

**Root cause**: `views/agent/input_transition.rs::InputTransitionState` only has
three variants: `Idle | Loading | Compacting`. There is no `Hiding` /
`Showing` phase, and `views/agent.rs:234-240` performs an atomic transition:

```rust
if is_busy {
    if self.spinner_started_at.is_none() { … = Some(Instant::now()); }
} else {
    self.spinner_started_at = None;   // ← spinner disappears instantly
}
```

The TS hide-pause-show sequence is entirely missing.

### 3.3 Regression C — cursor visible under spinner

**Root cause**: `codelet/fspec-tui/src/app/events.rs:184-186, 222-225` unconditionally
calls `frame.set_cursor_position` whenever `ViewMode::Agent` and
`agent.cursor_position()` returns `Some`. The cursor coordinates come from
`last_input_area` + `MultiLineInput::cursor()` — both populated even when the
spinner is painted on top of the input area.

ratatui leaves the terminal cursor at wherever `set_cursor_position` last placed it,
so the OS draws its native cursor block at column 2 (just past the `> ` prompt
prefix), which lands under the **T** of "Thinking…" → exactly the screenshot.

## 4. Fix plan (for `testing`/`implementing` phases)

### 4.1 Regression A — independent spinner tick

- **Don't** require an `Action::SpinnerTick`. Simpler: drop the `should_render` gate
  while busy.
- In `app/events.rs::run`, change the render-tick arm to:

  ```rust
  _ = tick.tick() => {
      let busy = /* read session_status of focused session */;
      if self.should_render || busy {
          guard.terminal().draw(|frame| { … })?;
          self.should_render = false;
      }
  }
  ```

  Same fix applies during a finish animation (phase != `Idle`).
- Cap CPU when neither busy nor animating: `should_render = false` path is
  unchanged, ticks no-op.

### 4.2 Regression B — finish animation state machine

- Extend `InputTransitionState`:

  ```rust
  enum InputTransitionState {
      Idle,
      Loading { elapsed_ms: u64 },
      Compacting { elapsed_ms: u64 },
      Hiding   { captured: String, visible_chars: usize, started_at: Instant },
      Showing  { placeholder: String, visible_chars: usize, started_at: Instant, hide_completed_at: Instant },
  }
  ```

- Owner: `AgentView`. Drive transitions in `render_with_store` based on
  `(prev_status, new_status, now)`:
  - `Running|Compacting → Idle` → capture current spinner string,
    `state = Hiding { captured, visible_chars: captured.len(), started_at: now }`.
  - Each render frame in `Hiding`: `visible_chars = saturating_sub((elapsed / 17ms) * 5)`;
    when `visible_chars == 0`, mark `hide_completed_at = now`, **stay in Hiding** until
    `now - hide_completed_at >= 34ms`, then enter `Showing`.
  - In `Showing`: `visible_chars = min(placeholder.len(), (elapsed / 17ms) * 5)`;
    when `visible_chars == placeholder.len()`, transition to `Idle`.
- Short-circuit:
  - On `Hiding|Showing` + `Loading` trigger → reset to `Loading { elapsed_ms: 0 }`.
  - On `Hiding|Showing` + printable keystroke → force `Idle` and re-dispatch
    the keystroke into `MultiLineInput`.

### 4.3 Regression C — cursor suppression

- In `app/events.rs`, replace:

  ```rust
  if let Some((x, y)) = self.navigator.agent.cursor_position() {
      frame.set_cursor_position((x, y));
  }
  ```

  with a guard that consults the *current* `InputTransitionState` (idle only):

  ```rust
  if self.navigator.agent.is_cursor_visible() {
      if let Some((x, y)) = self.navigator.agent.cursor_position() {
          frame.set_cursor_position((x, y));
      }
  }
  ```

- `AgentView::is_cursor_visible` returns `true` iff
  `current InputTransitionState == Idle && session_status not Running/Compacting`.

## 5. Test surface

| Layer | File | What it covers |
|-------|------|----------------|
| Unit | `views/agent/input_transition.rs` | `Hiding`/`Showing` math, phase advance, frame index, short-circuit |
| Unit | `views/agent/spinner.rs` | (unchanged — still passes) |
| Integration | `tests/agentview_thinking_animation_parity_rpc093.rs` (new) | end-to-end render sequence; cursor suppression; tick cadence (uses `tokio::time::pause()` clock manipulation) |
| Snapshot | `views/agent/input_transition.rs::tests` | `insta` snapshots of each phase frame |

## 6. Out of scope (still)

- Markdown / colour formatting inside thinking blocks (deferred).
- Compaction text in input placeholder (Rust uses the footer chip, per RPC-095 [13]).
- HITL placeholder/options (separate card).
- `Action::SpinnerTick`-style explicit tick action — superseded by the
  always-redraw-while-busy approach in §4.1.
