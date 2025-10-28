@infrastructure
@integration-test
@concurrency
@file-ops
@critical
@LOCK-002
Feature: Implement file locking system (LockedFileManager + refactoring)
  """
  CRITICAL: This is an all-at-once migration. MUST work 100% perfectly the first time. Complete implementation includes: LockedFileManager + refactor utilities + refactor all ~50-60 commands + refactor TUI store + comprehensive concurrency tests. NO partial states, NO gradual rollout.
  Implementation phases: (1) Create LockedFileManager singleton 2-3h, (2) Refactor ensure-files.ts 1-2h, (3) Refactor all commands 3-4h, (4) Refactor TUI store 1h, (5) Comprehensive testing 2-3h. Total estimate: 9-13 hours.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Create src/utils/file-manager.ts with LockedFileManager singleton class (proper-lockfile + readers-writer pattern + atomic writes)
  #   2. Refactor src/utils/ensure-files.ts to use LockedFileManager for all ensure* functions (ensureWorkUnitsFile, ensureTagsFile, ensureFoundationFile, ensurePrefixesFile, ensureEpicsFile, ensureExampleMapFile, ensureHooksFile)
  #   3. Refactor all 54 command files that import from ensure-files.ts to use fileManager.transaction() for read-modify-write operations
  #   4. Refactor src/tui/store/fspecStore.ts to use LockedFileManager for all JSON file reads (TUI refresh operations)
  #   5. All read-modify-write operations MUST use fileManager.transaction() with mutation-based callback API
  #   6. ensure* functions MUST use read-lock-first pattern: attempt READ lock, on ENOENT upgrade to WRITE lock with double-check
  #
  # EXAMPLES:
  #   1. NEW FILE: src/utils/file-manager.ts - Create LockedFileManager singleton with readJSON(), writeJSON(), transaction() methods
  #   2. REFACTOR: src/utils/ensure-files.ts - All ensure* functions use fileManager.transaction() internally
  #   3. REFACTOR: src/tui/store/fspecStore.ts - TUI store uses fileManager.readJSON() for all JSON reads (no writes, TUI is read-only)
  #   4. REFACTOR: Work unit commands (20 files): update-work-unit.ts, update-work-unit-status.ts, create-task.ts, create-story.ts, create-bug.ts, delete-work-unit.ts, prioritize-work-unit.ts, repair-work-units.ts, validate-work-units.ts, set-user-story.ts, add-dependency.ts, remove-dependency.ts, clear-dependencies.ts, add-attachment.ts, remove-attachment.ts, add-architecture-note.ts, remove-architecture-note.ts, add-rule.ts, remove-rule.ts, add-example.ts, remove-example.ts, add-question.ts, remove-question.ts, answer-question.ts, add-assumption.ts
  #   5. REFACTOR: Virtual hook commands (4 files): add-virtual-hook.ts, remove-virtual-hook.ts, clear-virtual-hooks.ts, copy-virtual-hooks.ts, list-virtual-hooks.ts
  #   6. REFACTOR: Prefix and epic commands (2 files): update-prefix.ts, create-prefix.ts
  #   7. REFACTOR: Tag commands (3 files): register-tag.ts, validate-tags.ts, list-tags.ts
  #   8. REFACTOR: Foundation commands (3 files): update-foundation.ts, show-foundation.ts, add-diagram.ts
  #   9. REFACTOR: Example mapping commands (3 files): generate-scenarios.ts, export-example-map.ts, import-example-map.ts
  #   10. REFACTOR: Query/reporting commands (4 files): query-example-mapping-stats.ts, query-orphans.ts, query-dependency-stats.ts, suggest-dependencies.ts, export-dependencies.ts, list-work-units.ts, list-attachments.ts
  #   11. REFACTOR: Board command (1 file): display-board.ts
  #   12. CHECK: src/index.ts - May have direct JSON file access that needs refactoring
  #   13. CHECK: src/utils/work-unit-tags.ts - May have direct JSON file access that needs refactoring
  #   14. NEW TEST: src/utils/__tests__/file-manager.test.ts - Comprehensive unit tests for LockedFileManager (concurrency, retries, timeouts, atomic writes, readers-writer pattern)
  #   15. UPDATE TEST: src/utils/__tests__/ensure-files.test.ts - Update tests to verify ensure* functions use fileManager.transaction()
  #   16. UPDATE TEST: All command test files (50+ files) - Update mocks to use fileManager instead of direct file operations
  #   17. UPDATE TEST: src/tui/__tests__/fspecStore-*.test.ts - Update TUI store tests to verify fileManager.readJSON() usage
  #   18. UPDATE: package.json - Add proper-lockfile dependency (~4.1.2)
  #   19. UPDATE: package.json - Add @types/proper-lockfile dev dependency
  #   20. CHECK: src/hooks/integration.ts, src/hooks/config.ts, src/hooks/script-generation.ts - May have JSON file access that needs refactoring
  #   21. CHECK: src/validators/validate-json-schema.ts, src/validators/generic-foundation-validator.ts, src/validators/json-schema.ts - May have JSON file access that needs refactoring
  #
  # ========================================
  Background: User Story
    As a developer implementing file locking
    I want to refactor all JSON file operations to use LockedFileManager
    So that all file operations are protected by the three-layer locking system

  Scenario: Create LockedFileManager singleton with three-layer locking
    Given I need to implement file locking for concurrent access safety
    When I create src/utils/file-manager.ts
    Then the file should export a LockedFileManager singleton class
    And the class should implement readJSON() method for concurrent reads
    And the class should implement writeJSON() method for exclusive writes
    And the class should implement transaction() method for atomic read-modify-write
    And the class should use proper-lockfile for inter-process coordination
    And the class should use readers-writer pattern for in-process optimization
    And the class should use atomic write-replace pattern for safe writes

  Scenario: Refactor ensure-files.ts to use LockedFileManager
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor src/utils/ensure-files.ts
    Then all ensure* functions should use fileManager.readJSON() for reads
    And ensureWorkUnitsFile should use read-lock-first pattern
    And ensureTagsFile should use read-lock-first pattern
    And ensureFoundationFile should use read-lock-first pattern
    And ensurePrefixesFile should use read-lock-first pattern
    And ensureEpicsFile should use read-lock-first pattern
    And ensureExampleMapFile should use read-lock-first pattern
    And ensureHooksFile should use read-lock-first pattern
    And on ENOENT error functions should upgrade to WRITE lock with double-check

  Scenario: Refactor TUI store to use LockedFileManager for reads
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor src/tui/store/fspecStore.ts
    Then all JSON file reads should use fileManager.readJSON()
    And the TUI store should NEVER use fileManager.writeJSON() (read-only)
    And file watcher callbacks should use fileManager.readJSON()
    And loadData() method should use fileManager.readJSON() for all files

  Scenario: Refactor work unit commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor work unit command files
    Then update-work-unit.ts should use fileManager.transaction() for modifications
    And update-work-unit-status.ts should use fileManager.transaction() for status changes
    And create-task.ts should use fileManager.transaction() for creating tasks
    And create-story.ts should use fileManager.transaction() for creating stories
    And create-bug.ts should use fileManager.transaction() for creating bugs
    And delete-work-unit.ts should use fileManager.transaction() for deletions
    And prioritize-work-unit.ts should use fileManager.transaction() for reordering
    And repair-work-units.ts should use fileManager.transaction() for repairs
    And validate-work-units.ts should use fileManager.readJSON() for validation (read-only)
    And set-user-story.ts should use fileManager.transaction() for user story updates
    And all dependency commands should use fileManager.transaction()
    And all attachment commands should use fileManager.transaction()
    And all architecture note commands should use fileManager.transaction()
    And all example mapping commands (add-rule, add-example, add-question, answer-question, add-assumption) should use fileManager.transaction()

  Scenario: Refactor virtual hook commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor virtual hook command files
    Then add-virtual-hook.ts should use fileManager.transaction() for adding hooks
    And remove-virtual-hook.ts should use fileManager.transaction() for removing hooks
    And clear-virtual-hooks.ts should use fileManager.transaction() for clearing hooks
    And copy-virtual-hooks.ts should use fileManager.transaction() for copying hooks
    And list-virtual-hooks.ts should use fileManager.readJSON() for listing (read-only)

  Scenario: Refactor prefix and epic commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor prefix and epic command files
    Then update-prefix.ts should use fileManager.transaction() for modifications
    And create-prefix.ts should use fileManager.transaction() for creation

  Scenario: Refactor tag commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor tag command files
    Then register-tag.ts should use fileManager.transaction() for registering tags
    And validate-tags.ts should use fileManager.readJSON() for validation (read-only)
    And list-tags.ts should use fileManager.readJSON() for listing (read-only)

  Scenario: Refactor foundation commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor foundation command files
    Then update-foundation.ts should use fileManager.transaction() for updates
    And show-foundation.ts should use fileManager.readJSON() for display (read-only)
    And add-diagram.ts should use fileManager.transaction() for adding diagrams

  Scenario: Refactor example mapping commands to use transaction() pattern
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor example mapping command files
    Then generate-scenarios.ts should use fileManager.readJSON() for reading example maps
    And export-example-map.ts should use fileManager.readJSON() for exporting
    And import-example-map.ts should use fileManager.transaction() for importing

  Scenario: Refactor query and reporting commands to use LockedFileManager
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor query and reporting command files
    Then query-example-mapping-stats.ts should use fileManager.readJSON() (read-only)
    And query-orphans.ts should use fileManager.readJSON() (read-only)
    And query-dependency-stats.ts should use fileManager.readJSON() (read-only)
    And suggest-dependencies.ts should use fileManager.readJSON() (read-only)
    And export-dependencies.ts should use fileManager.readJSON() (read-only)
    And list-work-units.ts should use fileManager.readJSON() (read-only)
    And list-attachments.ts should use fileManager.readJSON() (read-only)

  Scenario: Refactor board command to use LockedFileManager
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I refactor src/commands/display-board.ts
    Then the command should use fileManager.readJSON() for all file reads (read-only)

  Scenario: Check and refactor src/index.ts if needed
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I audit src/index.ts for direct JSON file access
    Then any readFile or writeFile calls on JSON files should be refactored to use fileManager

  Scenario: Check and refactor src/utils/work-unit-tags.ts if needed
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I audit src/utils/work-unit-tags.ts for direct JSON file access
    Then any readFile or writeFile calls on JSON files should be refactored to use fileManager

  Scenario: Check and refactor hooks files if needed
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I audit src/hooks/integration.ts, src/hooks/config.ts, src/hooks/script-generation.ts
    Then any readFile or writeFile calls on JSON files should be refactored to use fileManager

  Scenario: Check and refactor validator files if needed
    Given I have src/utils/file-manager.ts with LockedFileManager singleton
    When I audit src/validators/validate-json-schema.ts, src/validators/generic-foundation-validator.ts, src/validators/json-schema.ts
    Then any readFile or writeFile calls on JSON files should be refactored to use fileManager

  Scenario: Create comprehensive unit tests for LockedFileManager
    Given I have implemented src/utils/file-manager.ts
    When I create src/utils/__tests__/file-manager.test.ts
    Then the test file should have concurrency tests for multiple readers
    And the test file should have concurrency tests for reader+writer blocking
    And the test file should have concurrency tests for multiple writers blocking
    And the test file should have retry logic tests with exponential backoff
    And the test file should have timeout tests for lock acquisition
    And the test file should have atomic write-replace tests
    And the test file should have readers-writer pattern tests
    And the test file should have stale lock detection tests
    And the test file should have transaction rollback tests on error

  Scenario: Update ensure-files.test.ts to verify LockedFileManager usage
    Given I have refactored src/utils/ensure-files.ts to use LockedFileManager
    When I update src/utils/__tests__/ensure-files.test.ts
    Then tests should verify ensure* functions use fileManager.transaction()
    And tests should verify read-lock-first pattern is used
    And tests should verify ENOENT triggers WRITE lock upgrade

  Scenario: Update all command test files to use LockedFileManager mocks
    Given I have refactored all command files to use LockedFileManager
    When I update all command test files (50+ files)
    Then mocks should use fileManager.readJSON() instead of readFile
    And mocks should use fileManager.writeJSON() instead of writeFile
    And mocks should use fileManager.transaction() for read-modify-write tests

  Scenario: Update TUI store tests to verify LockedFileManager usage
    Given I have refactored src/tui/store/fspecStore.ts to use LockedFileManager
    When I update src/tui/__tests__/fspecStore-*.test.ts
    Then tests should verify all file reads use fileManager.readJSON()
    And tests should verify TUI store NEVER uses fileManager.writeJSON()

  Scenario: Add proper-lockfile dependency to package.json
    Given I need proper-lockfile for inter-process locking
    When I update package.json
    Then the dependencies section should include "proper-lockfile": "~4.1.2"

  Scenario: Add @types/proper-lockfile dev dependency to package.json
    Given I need TypeScript types for proper-lockfile
    When I update package.json
    Then the devDependencies section should include "@types/proper-lockfile": "latest"
