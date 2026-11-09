@BUG-582
@done
@rust
@coverage-tracking
@cli
@BUG-557
Feature: Coverage files hold duplicate test mappings
  """
  An exact duplicate is a second test mapping of one scenario with the same file and the same lines text, or a second implementation mapping of one test mapping with the same file and the same line list. link-coverage (fspec-core link_coverage.rs) merges into the existing mapping instead of appending one; fspec validate (validate_coverage_drift.rs, BUG-485) reports exact duplicates; scripts/merge-duplicate-coverage-mappings.py merged the 428 duplicates this repository held (362 of them carried different implementation mappings, which are joined per file). Same-file mappings with different ranges stay allowed; the exact @step checks cover them. Tests: rust/fspec-core/tests/bug557_coverage_files_hold_each_mapping_once.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An exact duplicate is a test mapping with the same file and the same lines text as another test mapping of the same scenario; an exact duplicate implementation mapping has the same file and the same line list as another in the same test mapping. Same-file mappings with different ranges stay allowed
  #   2. link-coverage never creates an exact duplicate: linking a test file and range the scenario already maps adds nothing, and an implementation mapping given with it is added to the existing mapping
  #   3. fspec validate reports every exact duplicate test or implementation mapping with the coverage file, the scenario and the mapping, and exits 1
  #   4. scripts/merge-duplicate-coverage-mappings.py is the one-off cleanup: it merges each set of exact duplicate test mappings into the first, joining their implementation mappings (one per file, lines united), and changes nothing else
  #
  # EXAMPLES:
  #   1. Scenario 'Log in' maps tests/login.rs:1-5 and link-coverage links tests/login.rs:1-5 again with src/login.rs:2-3: the scenario still has one tests/login.rs:1-5 mapping, now carrying src/login.rs lines 2-3
  #   2. A coverage file whose scenario 'Log in' maps tests/login.rs:1-5 twice makes fspec validate report the duplicate and exit 1; tests/login.rs:1-5 and tests/login.rs:7-9 pass
  #   3. The cleanup script turns two tests/login.rs:1-5 mappings carrying src/a.rs [2] and src/a.rs [3], src/b.rs [1] into one mapping carrying src/a.rs [2,3] and src/b.rs [1]
  #
  # ========================================
  Background: User Story
    As a maintainer keeping coverage files accurate
    I want to have each test mapping recorded once per scenario
    So that re-pointing a range updates the one mapping instead of leaving a stale copy

  Scenario: Linking an already mapped test range merges into the existing mapping
    Given scenario "Log in" maps tests/login.rs:1-5
    When link-coverage links tests/login.rs:1-5 again with the implementation src/login.rs lines 2-3
    Then the scenario still has one tests/login.rs:1-5 mapping, and it carries src/login.rs lines 2-3

  Scenario: Validate reports an exact duplicate test mapping
    Given scenario "Log in" of login.feature.coverage maps tests/login.rs:1-5 twice
    When fspec validate runs
    Then it reports the duplicate tests/login.rs:1-5 mapping of scenario "Log in" in spec/features/login.feature.coverage and exits 1

  Scenario: Validate reports an exact duplicate implementation mapping
    Given a test mapping of scenario "Log in" carries the implementation src/login.rs lines 2-3 twice
    When fspec validate runs
    Then it reports the duplicate src/login.rs implementation mapping and exits 1

  Scenario: Same-file mappings with different ranges pass
    Given scenario "Log in" maps tests/login.rs:1-5 and tests/login.rs:7-11
    When fspec validate runs
    Then it reports no duplicate mapping and exits 0

  Scenario: The cleanup script merges existing duplicates
    Given two tests/login.rs:1-5 mappings of one scenario carry src/a.rs [2] and src/a.rs [3] with src/b.rs [1]
    When scripts/merge-duplicate-coverage-mappings.py runs in the project
    Then one tests/login.rs:1-5 mapping remains, carrying src/a.rs [2, 3] and src/b.rs [1], and nothing else in the file changes
