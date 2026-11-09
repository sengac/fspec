@done
@BUG-591
@cli
@coverage-tracking
@rust
Feature: fspec audit --fix records line text only where git shows the lines did not move
  """
  COV-079: an implementation mapping records the trimmed text of its first
  and last line (firstLine, lastLine). For a legacy mapping without that text,
  fspec audit --fix used to record the text at the mapping's current lines,
  which froze a wrong mapping in place when the code had already moved. Now
  --fix records the text only when git shows the lines cannot have moved:
  lines 1 through the mapping's last line read exactly as they did at the
  commit that linked the mapping. The linking commit is the oldest commit,
  walking back from HEAD through the commits that changed the coverage file
  (first parent), from which every later version of the coverage file holds
  the same mapping (scenario, test file, implementation file, line list). The
  current content is the working tree's file, or the staged file under
  --changed. Otherwise the problem stays impl-text-unrecorded with fix kind
  guided, and its fact names why: the file changed since the linking commit,
  the mapping is not committed, or the project is not a git working tree.
  Exact facts only: fspec compares lines at commits git names and never
  guesses where code moved. Core: rust/fspec-core/src/commands/audit/impl_ranges.rs
  and rust/fspec-core/src/commands/audit/history.rs. Tests:
  rust/fspec-core/tests/bug591_audit_fix_records_text_only_where_lines_did_not_move.rs.
  """

  Background: User Story
    As a AI agent running fspec audit --fix
    I want to have --fix record a legacy implementation mapping's line text only when git shows its lines cannot have moved
    So that a mapping whose code already moved is reported for relinking instead of being frozen in place with the wrong text

  Scenario: A legacy mapping whose file is unchanged since its linking commit gets its text recorded
    Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    And src/login.rs is unchanged since commit A
    When fspec audit --fix runs
    Then the mapping records the text of lines 10 and 24 as firstLine and lastLine

  Scenario: A legacy mapping whose file changed above it is reported, not recorded
    Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    And two lines were added above line 10 of src/login.rs after commit A
    When fspec audit --fix runs
    Then the mapping still records no line text
    And fspec audit reports it as impl-text-unrecorded with fix kind "guided"
    And its fact says src/login.rs changed since the mapping was linked in commit A

  Scenario: Lines added only after the mapping do not stop the recording
    Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    And lines were added only after line 24 of src/login.rs since commit A
    When fspec audit --fix runs
    Then the mapping records the text of lines 10 and 24 as firstLine and lastLine

  Scenario: A later commit of the coverage file for another scenario keeps the linking commit
    Given commit A links "Log in" to src/login.rs lines 10-24 without line text
    And two lines were added above line 10 of src/login.rs after commit A
    And commit B changes only the mapping of scenario "Log out" in the same coverage file
    When fspec audit --fix runs
    Then the mapping still records no line text
    And its fact names commit A as the linking commit

  Scenario: An uncommitted legacy mapping is reported, not recorded
    Given the coverage file links "Log in" to src/login.rs lines 10-24 without line text only in the working tree
    When fspec audit --fix runs
    Then the mapping still records no line text
    And its fact says the mapping is not committed

  Scenario: Outside a git working tree a legacy mapping is never recorded
    Given a project that is not a git working tree whose coverage links "Log in" to src/login.rs lines 10-24 without line text
    When fspec audit --fix runs
    Then the mapping still records no line text
    And its fact says the project is not a git working tree
