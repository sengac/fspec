# BUG-157: `fspec format` duplicates scenarios when a Background section is present

## Reproduction (minimal, 2-scenario file)

Input file:

```gherkin
@test
Feature: Formatter Background duplication repro

  Background: User Story
    As a user
    I want to test
    So that I can verify

  Scenario: First scenario
    Given a precondition
    When I do the action
    Then I see the result

  Scenario: Second scenario
    Given another precondition
    When I do another action
    Then I see another result
```

Command: `fspec format <file>`

Result: `grep -c 'Scenario:'` reports **4** instead of 2. The formatter re-appends
every scenario block indented under the Background section:

```diff
8a9,18
>     Scenario: First scenario
>     Given a precondition
>     When I do the action
>     Then I see the result
>
>     Scenario: Second scenario
>     Given another precondition
>     When I do another action
>     Then I see another result
```

## Observations

- Files **without** a `Background:` section are unaffected (2 scenarios stay 2).
- The pipe `|` character in step text was investigated and ruled out as the
  trigger (a file with `|` in a step and no Background formats cleanly).
- Re-running `fspec format` on an already-formatted file is **not idempotent** —
  it keeps appending another copy (the UPD-003 feature file grew to 40 scenario
  blocks after repeated format runs).
- The duplicated blocks are indented 4 spaces (nested under Background), while
  the originals are 2-space top-level.
- The formatted output still passes `fspec validate` (Gherkin parser accepts
  scenarios nested under Background), so the duplication is silent.
- The `check` command's formatting check fails on the file forever, because the
  formatter never produces a stable output for Background-containing files.

## Impact

- Any feature using the standard `Background: User Story` layout (which
  `generate-scenarios` produces) fails the formatting check permanently.
- Work units cannot complete the "format with fspec format" step without
  manually editing the file back.
- Workaround used for UPD-003: write the file with clean 2-space indentation
  and skip `fspec format` (file passes `fspec validate`).

## Suggested fix direction

The formatter's Gherkin re-serialization appears to emit Background content and
then re-emit the scenario blocks as if they were part of the Background body.
Likely cause: the Background node's children include the trailing scenarios
(the parser/formatter treats everything after `Background:` at the same or
deeper indent as Background content), and the serializer prints both the
Background's children and the top-level scenario list. Check the Background
node construction in the Gherkin parse/serialize path in `codelet-fspec-core`
(formatter module).
