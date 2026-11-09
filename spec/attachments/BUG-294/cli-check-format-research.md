# Research: `fspec check` fails on single-line feature tags (`cli_check`)

Target: `cargo test --profile ci-test -p codelet-fspec --test cli_check`
Result on current tree **and on clean HEAD**: 3 passed, 2 failed
(`scenario_cli_passes_and_exits_0_for_valid_registered_feature_files` l.118,
`scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher` l.201 — both expect exit 0, get 1).

## Reproduction (debug binary, 2026-10-30)
Fixture used by the test (`valid_feature`): tags on one line.
```
@comp @grp
Feature: A

  Scenario: A
    Given x
```
`fspec check --verbose`:
```
Gherkin syntax: PASS
Tag validation: PASS
Formatting: FAIL
Errors:
  - Formatting check failed: spec/features/a.feature needs formatting
✗ Some checks failed        (exit 1)
```
`fspec format spec/features/a.feature` rewrites the tags one per line:
```
@comp
@grp
Feature: A
```

## Analysis
The Rust formatter emits one tag per line; `check` treats any formatter delta as a failure.
The test fixture predates that formatter convention. Note open bug **BUG-152** ("Rust-port
`format` corrupts feature files…") concerns the same formatter; check whether the
one-tag-per-line output is intended TS parity before changing either side.

## Suggested fix
Decide the canonical tag layout (TS `fspec format` reference). If one-tag-per-line is correct,
update the `cli_check` fixture (and feature step text if it embeds the layout). If not, fix the
formatter to keep same-line tags and add a regression test.
