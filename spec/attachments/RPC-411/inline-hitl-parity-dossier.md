# RPC-411 — Inline HITL Prompt Parity Dossier

**Goal:** total UX parity with the TypeScript Ink implementation of `request_user_input`.
The Critical-priority modal `HitlDialog` is REPLACED by an inline prompt rendered in the composer
slot, exactly where the RPC-406 tool-approval pause prompt renders. Architecture mandate:
RPC-002 `spec/attachments/RPC-002/09-dialog-and-input-priority-port-spec.md` §A.2 ("HITL prompts →
High", only modal dialogs are Critical) and §C.5 (inline prompts embed in the layout, not a popup).

Depends on RPC-410 (wire protocol carries full questions + `{cancelled, answers}` response).

---

## 1. TypeScript reference — the exact UX to replicate

Files: `src/tui/components/InputTransition.tsx` (:385-464 render),
`src/tui/hooks/useHitlInput.ts` (entire file — the state machine),
`src/tui/components/AgentView.tsx` (:1312-1324, :5494-5499 wiring),
`src/tui/components/MultiLineInput.tsx` (freeform input reference).

### 1.1 Placement & priority
- Rendered INLINE in the input area (after the green `> ` prompt), replacing the MultiLineInput.
  No modal, no border, no Clear. Guard: `isPaused && hitlRequest && questions.length > 0`
  (`InputTransition.tsx:385-388`). HITL takes priority over stale tool-pause info.
- Input handler at HIGH priority (`useHitlInput.ts:85, 153` — `InputPriority.HIGH`,
  active only while paused with a request).

### 1.2 Options-question layout (`InputTransition.tsx:427-463`)
```
⏸ [1/3] Header: Question text?
 ● Option A — description of A
 ○ Option B — description of B
 ○ Other...
 (↑/↓ Navigate | Enter Select | Esc Cancel)
```
- `⏸ ` glyph magenta (:430). `[n/m]` magenta, ONLY when total questions > 1 (:431-433).
- `header` bold, then literal `: `, then plain question text (:434-436).
- Options: radio `' ● '` selected / `' ○ '` unselected; radio+label GREEN when selected, WHITE
  otherwise; description dim as ` — {description}` (:440-449).
- Virtual **"Other..."** entry ALWAYS appended after real options (:450-456): radio green/white by
  selection; label `Other...` dim + italic.
- Footer dim: ` (↑/↓ Navigate | Enter Select | Esc Cancel)` (:459-461).
- NO inverse-video highlight, NO hotkey letters, NO borders.

### 1.3 Freeform / Other-mode layout (`InputTransition.tsx:393-425`)
Shown when `(!hasOptions && freeformActive) || (hasOptions && otherActive)`:
```
⏸ [2/3] Header: Question text? (Enter Submit | Esc Cancel)
  ⚠ Please type a response or press Esc to go back
<MultiLineInput placeholder="Type your answer...">
```
- Same magenta `⏸ `/`[n/m]`, bold header, `: `, question.
- Hint dim, appended on the SAME header line: `' (Enter Submit | Esc Back to options)'` in Other
  mode; `' (Enter Submit | Esc Cancel)'` in plain freeform mode (:405-409).
- Empty-submit warning YELLOW: `  ⚠ Please type a response or press Esc to go back`, only while
  `showEmptyHint` (:411-413).
- Below: a REAL MultiLineInput with `placeholder="Type your answer..."`, sharing the composer's
  `value`/`onChange`/`onSubmit` (:414-422). **The composer draft becomes the initial answer text.**

### 1.4 Key handling (`useHitlInput.ts:153-262`)
| Key | Options question | Freeform question | Other mode |
|---|---|---|---|
| ↑ / ↓ | move selection, WRAPPING; totalItems = options.len + 1 (Other...) (:210-222) | falls through to input | falls through to input |
| Enter | if Other... selected → enter Other mode (:229-234); else capture `{id, selected:[label]}` and advance-or-submit (:226-241) | capture `{id, selected:[], other:<input>}`, clear input, advance-or-submit (:242-251) | empty/whitespace → set showEmptyHint, reject (:183-188); else submit `{id, selected:[], other}`, clear input, advance (:189-199) |
| Esc | CANCEL whole request → send `{cancelled:true}` (:176-180, 103-114) | CANCEL whole request | back to options list; clears hint AND input value; does NOT cancel (:168-174) |
| typing | falls through, no input rendered → chars do nothing | reaches MultiLineInput; typing clears showEmptyHint (:201-208, 255-258) | same |
| Tab / numbers / scroll | NOT handled — no hotkeys, no Tab-cycle, no scroll-select | — | — |

### 1.5 Multi-question flow (`useHitlInput.ts:134-151`)
- One question at a time; `advanceOrSubmit` appends the answer; if more questions remain:
  index+1, selection reset to 0, exit Other mode, clear hint. On the LAST question: submit ALL
  accumulated answers as `{cancelled:false, answers:[...]}`.
- NO backward navigation. `[n/m]` only when m > 1.

### 1.6 State semantics
- Session stays Paused for the whole flow; UI clears when status returns Running/Idle.
- Freeform answers CONSUME the shared composer draft (cleared after capture, :196, :249).
- Esc-cancel does NOT clear remaining composer draft (:177-180).
- Hook state (index/selection/answers/otherActive/hint) resets whenever not-paused or no request
  (:92-101).

---

## 2. Current Rust state (what must change)

- `codelet/fspec-tui/src/components/hitl_dialog.rs` — Critical modal, cyan border, `[a]`-hotkeys,
  Tab cycle, scroll-select, always-visible free-text row with hand-rolled char buffer, Esc pops
  WITHOUT sending anything (backend stranded Paused). **DELETE this component** (like RPC-406
  deleted `pause_dialog.rs`) and remove `HITL_DIALOG_ID` plumbing.
- `codelet/fspec-tui/src/app/dispatch_pause_hitl.rs` — `handle_pause_chunk` (:55-112) fetches
  pause+hitl in parallel, HITL wins on tie (:94-101) → currently `Action::OpenHitlDialog` →
  `handle_open_hitl_dialog` (:154-161) pushes modal; `handle_hitl_submitted` (:233-252);
  `handle_pause_cleared` (:117-121) removes dialog on resume. Rewire all of this to a per-session
  HITL slot in the store (mirror the RPC-406 pause slot `store/agent_view/pause_state.rs:40-50`).
- RPC-406 inline pause prompt = the pattern to follow: state slot in `store/agent_view/`, key
  handling in `views/agent/pause_keys.rs`-style module, rendering swapped into the input area in
  `views/agent/` (InputTransition equivalent). HITL must take precedence over the pause slot when
  both are set (TS parity, `InputTransition.tsx:385-388`).
- Existing tests that PIN THE WRONG BEHAVIOR — rewrite as part of this card:
  - `hitl_dialog.rs` unit tests (esc_does_not_emit_submission, hotkeys, free-text row) — removed
    with the component; replace with inline-prompt tests.
  - `codelet/fspec-tui/tests/pause_hitl_rpc053.rs` — steps asserting the modal and
    "backend.send_hitl_response is NEVER called" on Esc (:305-310) — now Esc MUST send
    `{cancelled:true}`.
  - `codelet/fspec-tui/tests/agent_input_paste_routing_rpc403.rs:283-331` — "paste while HitlDialog
    is open never reaches agent input". New semantics: in options mode, paste must NOT reach the
    composer (consume/ignore); in freeform/Other mode paste GOES INTO the shared input (TS parity —
    the real MultiLineInput is active).

---

## 3. Freeform input: reuse the existing multiline stack (tui-textarea-informed)

The freeform/Other mode must reuse the EXISTING composer input component
(`views/agent/multiline_input.rs` + `multiline_wrap.rs` + `multiline_input_enter.rs` +
`multiline_input_paste.rs`), which is the tui-textarea-backed port (RPC-402..405) already at parity
with `src/tui/components/MultiLineInput.tsx`:
- cursor nav (←/→/↑/↓ with boundary fall-through), Home/End, word ops (Alt+←/→, Alt+Backspace),
  Shift/Alt+Enter newline, soft-wrap + auto-grow 1→6 rows, paste with CRLF normalization,
  placeholder rendering (dim + inverse cursor block), unicode-width cursor mapping.
- tui-textarea source is cloned at `/tmp/tui-textarea` for reference (DisplayTextBuilder /
  next_scroll_top patterns already transplanted in `multiline_wrap.rs`). Consult it if the HITL
  integration needs geometry helpers — do NOT fork a second input widget.

Integration requirements:
- HITL freeform mode renders the SAME TextArea state (shared composer draft) with placeholder
  swapped to `Type your answer...` — matching TS which passes the composer's value/onChange/onSubmit.
- Enter in freeform mode is intercepted by the HITL layer (submit answer) BEFORE the normal
  agent-submit path; Shift+Enter still inserts a newline (MultiLineInput parity — Enter submits,
  modified Enter = newline via `multiline_input_enter.rs`).
- History nav (Shift+↑/↓) and session switching (Shift+←/→) keys must not be broken while HITL is
  NOT active; while HITL freeform is active follow TS: handler intercepts only Enter/Esc and lets
  everything else fall through to the input.

---

## 4. Esc/cancel correctness (the stranding bug)

- Esc on an options question or plain freeform question sends the wire response
  `{cancelled: true, answers: []}` via `backend.send_hitl_response`, then clears the HITL slot.
  Backend: blocked `wait_for_hitl_response` returns `Cancelled{cancelled:true}`; tool reports
  cancellation to the LLM; status → Running; `PauseCleared` path clears any leftovers.
- Esc in Other mode: local only (back to options, clear hint + input). NOTHING sent.
- There must be NO code path that dismisses the HITL UI without either submitting answers or
  sending cancelled:true (the old modal's silent pop is the bug class fixed for pauses in RPC-406).

---

## 5. Acceptance criteria seeds (turn into rules/examples during Example Mapping)

1. HITL renders inline in the composer slot (no modal layer, no border) while status is Paused with
   a pending request; composer draft text + cursor survive an options-only HITL round-trip.
2. `[n/m]` progress shown only for multi-question requests; questions advance one at a time; all
   answers submitted together after the last question.
3. Options list: ●/○ radios, green selected, dim ` — description`, dim-italic `Other...` appended,
   dim footer `(↑/↓ Navigate | Enter Select | Esc Cancel)`; ↑/↓ wrap across options+Other.
4. Enter on `Other...` enters Other mode; Esc in Other mode returns to options (clears hint+input);
   empty Enter in Other mode shows the yellow ⚠ hint; typing clears the hint.
5. Freeform mode reuses the composer multiline input with placeholder `Type your answer...`; the
   pre-existing composer draft is the initial answer text and is cleared when the answer is captured.
6. Esc (not in Other mode) sends `{cancelled:true}` — verified at the backend boundary (mock backend
   records the call) — and the UI clears.
7. No letter hotkeys, no Tab cycling, no scroll-select, no always-visible free-text row.
8. HITL wins over a simultaneous tool-pause state; `Running/Idle` state change clears a stale HITL
   prompt.
9. `hitl_dialog.rs` deleted; no construction site for the old modal remains in src/ (source-shape
   test, mirroring RPC-406's Action::PauseResumed lock).

## 6. Testing notes
- ratatui `TestBackend` + the existing app-with-mock-backend harness (see
  `tests/pause_hitl_rpc053.rs` for the driving pattern) + unit tests on the new state machine
  module. Follow RPC-406's feature/test structure (`spec/features/inline-tool-approval-pause-prompt.feature`).
- Keep files < 300 lines: state machine (store slot) / key handling / rendering as separate modules.
- Run from `codelet/`: `cargo test -p codelet-fspec-tui` + clippy + fmt. Tee output to a file;
  never pipe through head/grep. Full-suite check before done.
