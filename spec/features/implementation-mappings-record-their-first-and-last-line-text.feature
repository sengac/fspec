@done
@COV-079
@cli
@coverage-tracking
@rust
Feature: Implementation mappings record their first and last line text
  """
  Implementation mappings stored only line numbers, so a mapping that still
  lay inside its file but now pointed at other code went unnoticed (BUG-515
  rule 3). link-coverage now records the trimmed text of a mapping's lowest
  and highest line (firstLine, lastLine) and refuses a first line that is
  blank or not unique in the file. fspec audit reports a mapping whose
  recorded lines no longer read that text, and fspec audit --fix carries it
  through the git line diff from the commit that linked it (BUG-635; this
  replaced the text-only shift by one offset). Legacy mappings without text get it recorded by --fix when
  their first line is non-blank and unique. Only line text is compared, never
  symbols, so it works for any language.
  Tests: rust/fspec-core/tests/cov079_impl_mappings_record_line_text.rs.
  """

  Background: User Story
    As a maintainer relying on coverage mappings
    I want to have implementation mappings remember the exact text of their first and last lines
    So that a mapping that now points at other code is detected and, when the code only moved, shifted exactly

  Scenario: link-coverage records the first and last line text
    Given src/login.rs whose lines 10 to 24 hold the log_in function
    When the implementation lines 10-24 are linked to scenario "Log in"
    Then the mapping records firstLine "pub fn log_in(user: &User) -> Session {" and lastLine "}"

  Scenario: A blank first line is refused
    Given src/login.rs whose line 9 is blank
    When the implementation lines 9-24 are linked to scenario "Log in"
    Then link-coverage fails naming the blank first line
    And the coverage file is unchanged

  Scenario: A first line that is not unique is refused
    Given src/login.rs whose line 24 reads "}" like other lines of the file
    When the implementation lines 24-30 are linked to scenario "Log in"
    Then link-coverage fails naming how often the first line occurs
    And the coverage file is unchanged

  Scenario: A mapping whose code moved unchanged is shifted by fix
    Given a committed mapping src/login.rs 10-24 whose function moved unchanged to lines 30-44
    When fspec audit runs
    Then it reports "coverage impl-text-changed" for src/login.rs:10-24
    When fspec audit --fix runs
    Then the mapping becomes src/login.rs 30-44
    And it prints "src/login.rs:10-24 -> src/login.rs:30-44"
    And a second fspec audit --fix changes nothing

  Scenario: A mapping whose code moved and grew is not shifted
    Given a mapping src/login.rs 10-24 whose function moved to lines 30-47 and grew by 3 lines
    When fspec audit --fix runs
    Then the mapping stays src/login.rs 10-24
    And fspec audit still reports "coverage impl-text-changed" naming the link-coverage command

  Scenario: Legacy mappings get their text recorded only when the first line is unique
    Given a legacy mapping src/login.rs 10-24 without text whose line 10 is unique
    And a legacy mapping src/login.rs 9-24 without text whose line 9 is blank
    When fspec audit --fix runs
    Then the first mapping records its firstLine and lastLine
    And fspec audit reports only the second as "coverage impl-text-unrecorded"
