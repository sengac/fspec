# Research: six coverage rows with pre-existing lineage drift

Found by the REFAC-019/REFAC-020 lineage audit (2026-10-30), which compared every coverage
mapping touching files changed since `626eee1e` against that base commit. These rows were
already invalid at the base, so they were **preserved untouched** rather than guessed.

## A. Tests never carried the scenario's `@step` comments (native relinking refuses them)
| Feature | Scenario | Test file | Missing steps |
|---|---|---|---|
| model-selector-view | Close the model selector with Esc returns to Agent | `rust/fspec-tui/src/views/model_selector/mod.rs` | all 4 (Given in model selector / When Esc / Then closes / And returned to Agent view) |
| rpc013-source-shape | BoardView source contains the literal UnifiedBoardLayout footer string | `rust/fspec-tui/tests/source_shape_rpc013.rs` | `Then the file contains the substring "← →"`, `And the file contains the substring "Columns"` |
| rpc015-source-shape | New and modified board modules stay under 300 lines | `rust/fspec-tui/tests/source_shape_rpc015.rs` | `And views/board/ contains no file of 300 lines or more` |

Because `link-coverage` (correctly) enforces step validation for story/bug units, the
implementation mappings of these rows cannot be relocated after the navigator/board moves
until the tests carry the steps (or the steps are updated to what the tests now assert).

## B. Implementation lines already beyond end of file at the base commit
All map `rust/fspec-tui/src/views/navigator.rs` lines ≥ 329 (file was shorter at `626eee1e`):
- mux-close-session-landing — "back-to-board lands on the single Board view when…"
  (test `rust/fspec-tui/tests/bug175_mux_enabled_leak.rs`)
- mux-pane-esc-close — "Esc in the single Changed Files view still closes…"
- rust-mux-mode — "closing a session in mux mode retains the mux and…"

## Suggested fix
For A: add the missing `@step` comments where the test really asserts them, or update the
feature steps (via `fspec update-step`) to what the test asserts now; then relocate impl
mappings with `fspec show-coverage <f> --mapping-json` + `link-coverage --test-mapping-index
--expected-scenario-hash`. For B: locate the current implementation (navigator_mux_routing.rs /
navigator_mux_events.rs after REFAC-019) and relink precisely with the indexed selector.
The relocation helper used for REFAC-019 (content-matching against the base commit) is
described in the REFAC-019 closing architecture note.
