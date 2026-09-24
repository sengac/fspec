@cli
@validation
@coverage
@traceability
@wip
@AUDIT-002
Feature: Remediate coverage-mapping audit findings (dead mappings, divergent links, hygiene)

  """
  Source of truth for remediation: spec/attachments/AUDIT-002/fixlist.json (P1, 608 items grouped by impl file), problems.json.gz (P0/P2, 8,999 records). Re-link via the coverage link command (featureName + scenario + implFile/implLines + testFile/testLines); validate with show-coverage + audit-coverage + the project validation checks after each band.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work in priority order: P1 divergent/unverifiable re-links (live files, fixlist.json) first, then P0 dead-path migration/retirement (problems.json), then P2 hygiene
  #   2. P1 re-links must point at the specific function(s) that implement the scenario — whole-file blob mappings (all scenarios → the same large line range) are forbidden; every impl mapping must be a tight per-function range
  #   3. P0: a coverage file whose mappings are all dead gets either fresh Rust mappings (when a port exists) or is retired/archived — no coverage file may keep referencing deleted files
  #   4. After each remediation band, the project's coverage validation must pass: structural audit shows zero file-missing / beyond-EOF defects for the touched features, and no impl mapping exceeds 300 lines
  #
  # EXAMPLES:
  #   1. Maintainer opens fixlist.json, sees rust/tools/src/facade/wrapper.rs at the top with 21 divergent scenarios; re-links those 21 scenarios to the specific functions they exercise, then confirms the feature's audit verdict flips from divergent to aligned on re-audit
  #   2. Maintainer runs the structural audit after P0 work and sees the dead-file count for migrated features drop to zero — each previously-dead feature now either maps to its Rust port or is retired from the coverage set
  #
  # ========================================

  Background: User Story
    As a fspec maintainer
    I want to remediate the coverage mappings the audit flagged (dead paths, blob/divergent links, unlinked scenarios)
    So that show-coverage and audit reports reflect the Rust codebase and trust their coverage gates

  Scenario: Maintainer re-links divergent scenarios to their implementing functions
    Given a feature in the audit fix list whose scenarios were judged divergent or unverifiable
    And the flagged impl mappings are whole-file blobs (all scenarios mapped to the same large line range)
    When the maintainer re-links each flagged scenario to the specific function(s) that implement its behavior
    And the maintainer re-runs the semantic audit for that feature
    Then each re-linked scenario is judged aligned on re-audit
    And no impl mapping for the feature exceeds a single-function line range

  Scenario: Maintainer migrates or retires dead-path mappings
    Given a set of coverage files whose test or impl mappings reference files that no longer exist
    When the maintainer migrates each feature with a live Rust port to its new paths and line ranges
    And the maintainer retires each coverage file with no live mapping from the coverage set
    And the maintainer re-runs the structural audit
    Then zero file-missing defects remain for the migrated features
    And zero lines-past-EOF defects remain for the migrated features

