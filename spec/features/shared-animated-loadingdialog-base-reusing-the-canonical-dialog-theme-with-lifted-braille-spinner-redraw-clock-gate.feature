@ui-refinement
@tui
@done
@TUI-106
Feature: Shared animated LoadingDialog base reusing the canonical dialog_theme with lifted braille spinner + redraw-clock gate
  """
  UI/UX:
  - UI/UX: The loading indicator MUST be a dialog that extends the shared base dialog used by every other dialog in the program: components/dialog_theme.rs (RPC-027) FspecDialog + render_dialog — the rounded/black/accent visual contract. The LoadingDialog type builds a FspecDialog (Accent::Cyan, title, spinner row, optional '(idx/total)' counter row) and delegates pixel paint to render_dialog — the same way components/status_dialog.rs and components/checkpoint_restore_dialog.rs already do.
  Refactoring:
  - Lift the pure braille-dot spinner from views/agent/spinner.rs (RPC-095) to components/spinner.rs and re-export for agent code — avoids a views::agent dependency from the two mode views. Zero behavior change; the existing unit tests move and carry the contract byte-for-byte.
  Implementation:
  - View-owned modal (Pattern B) — the mode view owns an Option<LoadingDialog> and paints it over its panes in render() exactly like the existing RPC-365 restore modal, NOT through a Compositor Priority::Critical host like StatusDialog. TUI-106 delivers the shared primitives + the view loading fields (loading + load tracker + is_loading); the mode-view render/key wiring (dialog paint-over-panes, ESC guard) lands in TUI-107 (Checkpoints) / TUI-108 (Changed Files).
  - Shared staged LoadTracker (components/load_state.rs) folds the two views' existing stale-drop keys into a single stage-marker with the same key content ('files:{wu}:{name}', 'diff:{wu}:{name}:{path}', 'diff:{path}'). Labels live ON the tracker (keyed by stage): the view feeds its own text via begin_stage(key, label). complete_stage(key) is a no-op when the key does not match the current stage, preserving today's stale-async-drop behavior exactly. App dispatch stays the only spawn site; the view receives results only via the action bus (two front doors, one source of truth).
  Performance:
  - Redraw clock while a mode view is loading must extend the existing run-loop gate (App::is_session_busy / is_input_animating → tick_should_draw in app/mod.rs:84), NOT add a per-view tokio interval. Chain: view::is_loading() → Navigator::is_view_loading() (match on active_view) → App::is_view_loading() → fourth tick_should_draw operand. Clock cost is identical to a busy-session spinner; fully idle when the last stage flushes.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The loading state must be a dialog extending the shared base dialog (dialog_theme FspecDialog/render_dialog, Accent::Cyan) — never raw text, never a per-view ad-hoc popup
  #   2. Loading must be visually DISTINCT from empty: while a lazy load is in flight the view considers itself LOADING (is_loading true, empty false), never the real empty state — same contract as PROV-104's ModelSelector loaded/!loaded discriminator (state level here; pixel level in TUI-107/108)
  #   3. The loading dialog shows an ANIMATED braille spinner (lifted into components/spinner.rs from views/agent/spinner.rs, 10 frames at 80ms cadence) whose frames advance via the shared run-loop redraw gate — no per-view tokio timer
  #   4. For many-checkpoint repos the dialog shows STAGE-level progress: which of the cascade loads is in flight (list → files → diff for Checkpoints; list → diff for Changed Files), each with its own label fed by the view to the shared LoadTracker; per-item (idx/total) is a separate card TUI-109
  #   5. ESC is IGNORED while the loading dialog is active (mirrors StatusDialog rule [7]): the contract primitive is LoadingDialog::dismissable() == false — the full view-ESC wiring is TUI-107/108
  #
  # EXAMPLES:
  #   1. The checkpoints cascade lists: list… → files for <checkpoint>… → diff for <file>… — the spinner line at each stage names that stage
  #   2. A stale files result for a checkpoint the user no longer has selected must NOT clear the in-flight stage — the spinner keeps spinning
  #   3. While the mode-view reports loading, the run loop redraws every tick even with no input event; when the last stage flushes the clock goes idle again
  #
  # ========================================
  Background: User Story
    As a fspec TUI user on the board
    I want the open lazy mode-view (Checkpoints/Changed Files) to report clearly — via one shared loading dialog base — that it is loading, which stage of the cascade is running, and to animate while it does
    So that I am not lied to by a fake 'No results' empty state and a frozen UI, and the same shared primitive serves both mode views

  Scenario: Spinner frames live at components/spinner.rs and the agent view re-exports them byte-identically
    Given the braille spinner code lives in the shared components module
    When the frame picker is queried at elapsed times 0, 80 and 240 milliseconds
    Then the shared spinner returns the first braille glyph at 0 ms, the second glyph at 80 ms and the fourth glyph at 240 ms
    And the ten-frame table wraps back to the first glyph at 800 ms and continues at 80 ms cadence
    And the DIM-styled painter writes the first glyph at the row origin in its own area
    And the agent view's spinner module re-exports the exact same table, interval constant and painter so existing agent code compiles unchanged with byte-identical behavior

  Scenario: LoadingDialog extends the shared dialog base and paints a cyan loading popup over the mode view body
    Given a mode view body area at least 60 columns wide and 14 rows tall
    When a LoadingDialog titled "Loading checkpoints" with stage label "Loading checkpoint list…" is painted over that body at elapsed 0 milliseconds
    Then a centered popup with a rounded cyan border and the title "Loading checkpoints" is painted
    And a dialog row begins with the first braille glyph followed by the stage label
    And the counter row is absent while no progress has been reported
    And the pixel paint comes from the single shared dialog_theme render_dialog implementation the same way StatusDialog does

  Scenario: Loading is reported distinct from empty on both mode views
    Given a fresh Checkpoints view whose list load is in flight
    When the view state is inspected before any result has folded
    Then the view reports itself as loading and not empty
    And a fresh Changed Files view in the same state likewise reports itself as loading and not empty
    When the cascade list stage has flushed even though the result list is empty
    Then the view reports itself as not loading and empty so the real empty state can surface

  Scenario: The animated braille spinner advances between t=0 and t=80ms through the shared redraw gate
    Given the run loop draw gate is evaluated with should-render false, session not busy and input not animating
    When the active mode view is not loading
    Then the gate reports no redraw
    And when the active mode view reports loading the same gate reports a redraw
    And when a LoadingDialog is painted over the body at elapsed 0 milliseconds the first braille glyph appears in the dialog row
    And when the same dialog is painted at elapsed 80 milliseconds the second braille glyph appears instead
    And no per-view timer exists — the redraw decision comes only from the shared gate chain view is_loading up through navigator up through app to the draw guard

  Scenario: Each cascade stage of the Checkpoints and Changed Files views shows its own label
    Given a checkpoints cascade tracker is created with the list label "Loading checkpoint list…"
    When the files load for checkpoint cp-1 of work unit TUI-107 is requested with label "Loading files for X…"
    Then the active stage label is "Loading files for X…"
    And when that files stage completes with its own key and the diff load for file a.txt is requested with label "Loading diff for a.txt…"
    Then the active stage label is "Loading diff for a.txt…" and the view still reports itself as loading
    And when that diff stage completes with its own key
    Then no stage is active and the view reports itself as not loading
    And when a changed-files cascade tracker first labels the scan "Loading changed files…"
    And when the files list has flushed and a file diff for b.txt is requested with label "Loading diff for b.txt…"
    Then the active stage label is "Loading diff for b.txt…"

  Scenario: ESC cannot dismiss the loading contract while a load is in flight
    Given a LoadingDialog that represents an in-flight lazy load
    When the dismissability of the dialog is queried
    Then the dialog reports NOT dismissable
    And the StatusDialog anchor reports ESC is ignored in Restoring state so the shared keyboard contract that TUI-107/108 will wire is locked

  Scenario: A late stale result for a de-selected item does not clear the current stage's loading
    Given the checkpoints cascade tracker is in the files stage for checkpoint NEW
    When a stale files result for the earlier de-selected checkpoint OLD is folded with the OLD stage key
    Then the tracker is unchanged: still loading, active stage key still NEW
    And when the matching files result for NEW is folded with the NEW stage key
    Then the files stage completes and the cascade can advance to the diff stage

  Scenario: Open-stage cascade keys follow the existing stale-drop key shape
    Given a cascade files stage key is built for work unit AUTH-001 checkpoint pre-refactor
    Then the key is exactly "files:AUTH-001:pre-refactor"
    And a cascade diff stage key is exactly "diff:AUTH-001:pre-refactor:src/main.rs"
    And a changed-files diff stage key is exactly "diff:src/app.rs"
