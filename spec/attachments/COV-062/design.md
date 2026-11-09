# COV-062 design: cover codex-catalog-limit-resolution

## Problem

`spec/features/codex-catalog-limit-resolution.feature.coverage` had four scenarios with
empty `testMappings`, and `rust/sessions/tests/codex_catalog_limit_resolution.rs` had no
`@step` comments, so nothing proved which test verifies which step.

## What happened, in order

BUG-335 (1e5034db) had to move a card linked to this feature past `testing`; fspec's step
gate checks every scenario of a linked feature, and the `done` gate requires every scenario
covered. So BUG-335 already:

- split the test's setup into `write_catalog` / `create_codex_session` / `create_session`
  / `switch_model` so each Given/When step is a distinct statement, and added a
  byte-identical `// @step` comment for every step of all four scenarios;
- linked each scenario to its test function and to the implementation:
  `rust/providers/src/manager_runtime.rs:65-113` (limit resolution from the catalog),
  `rust/sessions/src/session_creation_helper.rs:239-240` (creation caches the limits),
  `rust/sessions/src/handle_impl/handle_impl_6.rs:5-95` (a switch re-resolves and caches),
  and `rust/providers/src/models/lookup.rs:8-18` for the partial-name scenario.

## This card

Verification and record only:

- a script compares every Gherkin step of the feature against the test's `// @step`
  comments: none missing, none extra;
- `fspec show-coverage codex-catalog-limit-resolution` reports 4/4 fully covered;
- the four tests pass (`cargo test --profile ci-test -p codelet-sessions --test
  codex_catalog_limit_resolution`, re-run in this card).

## Alternatives rejected

- Reverting BUG-335's mappings so this card could redo them: churn with no gain.
- `fspec generate-coverage`: forbidden; it guesses mappings.

## Wire / API changes

None.
