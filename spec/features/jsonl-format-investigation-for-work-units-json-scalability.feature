@scalability
@investigation
@performance
@file-ops
@medium
@TECH-001
Feature: JSONL format investigation for work-units.json scalability
  """
  Investigation focuses on JSONL (JSON Lines) format as alternative to monolithic JSON. JSONL stores each work unit as separate line (newline-delimited JSON). Uses Node.js fs.createReadStream for streaming reads, fs.appendFile for append-only writes. Benchmark using hyperfine for performance comparisons. Migration strategy: detect format, support both, migrate on demand. Git diff friendliness: one line per work unit change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. JSONL format must support append-only operations to avoid full file rewrites
  #   2. JSONL must be backward compatible with existing JSON format or provide migration path
  #   3. JSONL must provide better git diff output than monolithic JSON (show only changed lines)
  #   4. JSONL streaming reads must handle large datasets (1000+ work units) without loading entire file in memory
  #   5. Performance benchmarks must compare JSON vs JSONL for: read all, write one, update one, delete one operations
  #
  # EXAMPLES:
  #   1. Appending new work unit FEAT-100 to JSONL: write one line to file, no full rewrite needed
  #   2. Streaming read of 1000 work units from JSONL: memory usage stays constant at ~10MB regardless of dataset size
  #   3. Git diff shows: +{"id":"FEAT-100", ...} (one line added), not entire file change
  #   4. Migration command: fspec migrate-to-jsonl converts work-units.json to work-units.jsonl with backup
  #   5. Performance test: writing 1000 work units to JSONL takes 0.5s vs 10s for JSON (20x faster)
  #
  # ========================================
  Background: User Story
    As a developer working on large fspec projects
    I want to store work units efficiently without performance degradation
    So that the system remains responsive even with 1000+ work units

  Scenario: Append-only writes without full file rewrites
    Given work-units.jsonl exists with 100 work units
    When I append new work unit FEAT-100 to JSONL file
    Then only one line should be written to file
    And the entire file should NOT be rewritten
    And write operation should complete in less than 10ms

  Scenario: Streaming reads with constant memory usage
    Given work-units.jsonl contains 1000 work units
    When I stream read all work units from JSONL file
    Then memory usage should stay constant at approximately 10MB
    And memory usage should NOT scale with dataset size
    And all 1000 work units should be loaded successfully

  Scenario: Git diff shows only changed lines
    Given work-units.jsonl is tracked in git with 100 work units
    When I append new work unit FEAT-100 to JSONL file
    And I run git diff on work-units.jsonl
    Then diff should show only one line added: +{"id":"FEAT-100", ...}
    And diff should NOT show entire file as changed

  Scenario: Migration from JSON to JSONL with backup
    Given work-units.json exists with 50 work units
    When I run migration command: fspec migrate-to-jsonl
    Then work-units.jsonl should be created with 50 work units
    And work-units.json.backup should be created
    And all work unit data should be preserved without loss
    And migration should be reversible

  Scenario: Performance comparison shows JSONL is 20x faster for writes
    Given I have benchmark suite for write operations
    When I write 1000 work units to JSON format
    And I write 1000 work units to JSONL format
    Then JSONL writes should complete in approximately 0.5 seconds
    And JSON writes should complete in approximately 10 seconds
    And JSONL should be at least 10x faster than JSON
