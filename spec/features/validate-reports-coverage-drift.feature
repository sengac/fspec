@done
@rust
@validation
@cli
@BUG-485
Feature: fspec generate-coverage rewrites unrelated coverage files
  """
  Reproduced 2026-11-06 on a clean copy of HEAD's spec/ (git archive HEAD spec): 'fspec generate-coverage' (HEAD build) changed 3 files, all features whose coverage was out of sync with the feature file: structured-generation-aware-session-replay (3 scenarios missing from its coverage, last touched by 042d3974b), typed-durable-summary-identity (1 missing, a3389577f) and client-mcp-read-only (no coverage file at all, 2699693b7). It added the missing scenarios with empty mappings and recomputed stats; it changed no existing mapping. It also strips a trailing newline from a file it rewrites (write_json_atomic has none, TS parity) and leaves <file>.coverage.lock files behind (git-ignored). So generate-coverage does what its help says (update existing files with missing scenarios); the 'unrelated' rewrites come from cards that added scenarios without syncing coverage.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Product owner (2026-11-06): (a). Keep generate-coverage as it is, sync the coverage of the 3 features now, and make 'fspec validate' report features whose coverage file is missing scenarios (or lists scenarios the feature no longer has), so the drift is caught by the card that causes it. Exact detection only (scenario names), no semantic checks.
  #   2. Detection compares scenario names exactly (the names the Gherkin parser reads, counted, so duplicate names must match too); it reads no step text and makes no semantic comparison
  #   3. fspec validate checks coverage drift for every feature it validates (all features, or the one named), and a drifted coverage file makes it exit 1
  #
  # EXAMPLES:
  #   1. login.feature has scenarios 'Log in' and 'Log out' and its coverage file lists only 'Log in': fspec validate reports that the coverage file is missing scenario 'Log out' and exits 1
  #   2. login.feature.coverage lists 'Reset password', which login.feature no longer has: fspec validate reports that the coverage file lists scenario 'Reset password' the feature does not have and exits 1
  #   3. A feature with scenarios and no coverage file at all is reported as missing its coverage file; a feature whose coverage file lists exactly its scenario names passes
  #   4. After the 3 drifted features (structured-generation-aware-session-replay, typed-durable-summary-identity, client-mcp-read-only) are synced with generate-coverage --feature-name, fspec validate reports no coverage drift in this repository
  #
  # QUESTIONS (ANSWERED):
  #   Q: generate-coverage's whole-project run did exactly what its help documents: it added the scenarios that 3 features (structured-generation-aware-session-replay, typed-durable-summary-identity, client-mcp-read-only) had added without syncing their coverage, and changed no mapping. Which fix do you want? (a) keep generate-coverage as is, sync those 3 files now, and make 'fspec validate' report features whose coverage is missing scenarios so the drift is caught by the card that causes it; (b) make whole-project generate-coverage only create missing files and only report out-of-sync ones (changing them needs --feature-name); (c) both.
  #   A: Product owner (2026-11-06): (a). Keep generate-coverage as it is, sync the coverage of the 3 features now, and make 'fspec validate' report features whose coverage file is missing scenarios (or lists scenarios the feature no longer has), so the drift is caught by the card that causes it. Exact detection only (scenario names), no semantic checks.
  #
  # ========================================
  Background: User Story
    As a maintainer working on an fspec project
    I want to have fspec validate report coverage files that drifted from their feature's scenarios
    So that the card that adds or removes scenarios syncs the coverage, instead of a later generate-coverage changing unrelated files

  Scenario: A coverage file missing a scenario is reported
    Given login.feature has the scenarios "Log in" and "Log out" and its coverage file lists only "Log in"
    When fspec validate runs
    Then it reports that spec/features/login.feature.coverage is missing the scenario "Log out" and exits 1

  Scenario: A coverage file listing a scenario the feature no longer has is reported
    Given login.feature.coverage lists "Reset password", which login.feature no longer has
    When fspec validate runs
    Then it reports that spec/features/login.feature.coverage lists the scenario "Reset password" the feature does not have and exits 1

  Scenario: A feature without a coverage file yet is not reported
    Given login.feature has scenarios and no coverage file
    When fspec validate runs
    Then it reports no coverage drift for it and exits 0, because a feature gets its coverage file when its coverage is first generated

  Scenario: A coverage file that lists exactly the feature's scenarios passes
    Given login.feature.coverage lists exactly the scenario names of login.feature
    When fspec validate runs
    Then it reports no coverage drift and exits 0

  Scenario: The repository has no coverage drift
    Given the drifted coverage of structured-generation-aware-session-replay, typed-durable-summary-identity and client-mcp-read-only is synced with generate-coverage
    When fspec validate runs over this repository
    Then it reports no coverage drift
