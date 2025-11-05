@high
@cli
@migration
@data-model
@refactoring
@IDX-001
Feature: Implement Stable Indices with Soft Delete
  """
  Type definitions in src/types.ts: RuleItem, ExampleItem, QuestionItem, ArchitectureNoteItem interfaces extend base ItemWithId { id: number; text: string; deleted: boolean; createdAt: string; deletedAt?: string }
  Update WorkUnit interface in src/types.ts: Add nextRuleId: number, nextExampleId: number, nextQuestionId: number, nextNoteId: number fields (default 0 for backward compatibility)
  Remove command pattern: find item by ID, check if deleted, set deleted=true and deletedAt=new Date().toISOString(). Commands: remove-rule, remove-example, remove-question, remove-architecture-note, remove-attachment
  Add command pattern: create object { id: workUnit.nextRuleId++, text, deleted: false, createdAt: new Date().toISOString() }. Commands: add-rule, add-example, add-question, add-architecture-note
  Display filtering helper: filterActive(items) returns items.filter(item => !item.deleted). Used in show-work-unit, list-work-units, and all display commands
  Restore command structure: src/commands/restore-{rule,example,question,architecture-note}.ts. Find item by ID, validate exists, set deleted=false, delete deletedAt field. Support bulk IDs via comma-separated string
  Compact command: src/commands/compact-work-unit.ts. Filter out deleted items, sort by createdAt, renumber IDs sequentially (0, 1, 2...), reset ID counters to array.length. Require confirmation unless --force flag provided
  Show-deleted command: src/commands/show-deleted.ts. Display deleted items with IDs, text, deletedAt timestamps. Format: '[ID] text (deleted: timestamp)'. Useful for debugging and selective restoration
  Update show-work-unit command: Add --verbose flag to display createdAt and deletedAt timestamps. Show item count as 'X active items (Y deleted)' when deleted items exist. Display indices with gaps: [0], [2], [3] if [1] deleted
  Migration integration: Depends on MIG-001 migration system. Migration script src/migrations/migrations/001-stable-indices.ts converts string[] to ItemWithId[]. Called automatically by ensureLatestVersion() in src/utils/ensure-files.ts
  Auto-compact on done: Update src/commands/update-work-unit-status.ts. Before setting status='done', call compactWorkUnit(workUnit) to remove deleted items and renumber IDs. Ensures clean final state
  Help file structure: Create restore-{rule,example,question,architecture-note}-help.ts, compact-work-unit-help.ts, show-deleted-help.ts in src/commands/. Each exports CommandHelpConfig with comprehensive usage, examples, and AI guidance. Auto-loaded by help-registry.ts
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Indices must never shift when items are removed (stable IDs preserved)
  #   2. Remove operations must set deleted: true instead of splicing from array
  #   3. Display commands must filter deleted items and show stable IDs
  #   4. Item IDs must be auto-incrementing integers starting from 0
  #   5. Collections must use structure: {id: number, text: string, deleted: boolean, createdAt: string, deletedAt?: string}
  #   6. Work units must track nextRuleId, nextExampleId, nextQuestionId, nextNoteId counters
  #   7. Restore commands must set deleted: false and clear deletedAt timestamp
  #   8. Compact command must permanently remove deleted items and renumber IDs sequentially
  #   9. Auto-compact work unit when status changes to done (remove deleted items permanently)
  #   10. Questions must have both id and deleted fields (align with rules/examples/notes)
  #   11. Remove operations on already-deleted items must be idempotent (no error, return success)
  #   12. Migration script 001-stable-indices.ts must convert string arrays to object arrays with stable IDs
  #   13. Add commands must create objects with nextId++, deleted: false, createdAt: now()
  #   14. Display must show gaps in indices (e.g., [0], [2], [3] if [1] deleted) with deleted count
  #   15. Create restore command help files: src/commands/restore-rule-help.ts, restore-example-help.ts, restore-question-help.ts, restore-architecture-note-help.ts with CommandHelpConfig (auto-loaded by help-registry.ts)
  #   16. Create src/commands/compact-work-unit-help.ts with CommandHelpConfig for 'fspec compact-work-unit' command (auto-loaded by help-registry.ts)
  #   17. Update src/commands/bootstrap.ts to document stable indices, restore commands, compact command, and auto-compact behavior in workflow documentation
  #   18. Update src/help.ts and related help files to explain stable indices concept (items have IDs, removal is soft-delete, indices shown in display)
  #   19. Restore operations on non-deleted items must be idempotent (no error, return success with message 'Item ID already active')
  #   20. Restore operations on non-existent IDs must fail with clear error message 'Item ID does not exist'
  #   21. Compact operation must preserve chronological order of remaining items (sort by createdAt timestamp)
  #   22. ID counters (nextRuleId, etc.) must reset to count of remaining items after compaction (e.g., 5 items → nextId = 5)
  #   23. Display commands must show item count with format: 'X active items (Y deleted)' when deleted items exist
  #   24. Migration from v0.6.0 must initialize all ID counters to array.length (nextRuleId = rules.length, etc.)
  #   25. Type definitions must be updated: RuleItem, ExampleItem, QuestionItem, ArchitectureNoteItem interfaces with {id, text, deleted, createdAt, deletedAt?}
  #   26. All commands using .splice() must be replaced with soft-delete pattern (set deleted: true, deletedAt: ISO timestamp)
  #   27. WorkUnit interface must add nextRuleId, nextExampleId, nextQuestionId, nextNoteId fields (all number type, default 0)
  #   28. Update commands (update-rule, update-example, update-question, update-architecture-note) must use stable IDs instead of indices
  #   29. Yes, create 'fspec show-deleted <work-unit-id>' command showing deleted items with IDs, text, and deletedAt timestamps for debugging and selective restoration
  #   30. Require confirmation with --force flag to skip. Compaction is permanent and destructive - user should confirm before proceeding
  #   31. Yes, support bulk restore with comma-separated IDs: 'fspec restore-rule AUTH-001 2,5,7'. Validates all IDs before restoring any
  #   32. Yes, add --verbose flag to show-work-unit displaying createdAt and deletedAt timestamps for all items (active and deleted)
  #
  # EXAMPLES:
  #   1. AI removes rules at indices 1 and 2 sequentially: First removal sets rules[1].deleted=true (index stays), second removal sets rules[2].deleted=true (correct item removed, no shift)
  #   2. Work unit has 5 rules [0-4], AI removes indices 1 and 3, display shows: [0] Rule A, [2] Rule C, [4] Rule E with '3 active items (2 deleted)'
  #   3. User runs 'fspec restore-rule AUTH-001 2', rule[2].deleted changes from true to false, deletedAt field cleared, rule reappears in display at original index [2]
  #   4. Work unit with 10 rules (IDs 0-9), 4 deleted, user runs 'fspec compact-work-unit AUTH-001', deleted items removed permanently, remaining 6 rules renumbered to IDs 0-5, nextRuleId reset to 6
  #   5. Work unit at 'implementing' status has 3 deleted rules, user runs 'fspec update-work-unit-status AUTH-001 done', auto-compact triggers before status change, deleted rules removed, IDs renumbered, then status updated
  #   6. Migration converts rules: ['Rule A', 'Rule B'] to [{id: 0, text: 'Rule A', deleted: false, createdAt: '2025-01-31T12:00:00Z'}, {id: 1, text: 'Rule B', deleted: false, createdAt: '2025-01-31T12:00:00Z'}], nextRuleId set to 2
  #   7. User runs 'fspec add-rule AUTH-001 "New rule"', system creates {id: nextRuleId, text: 'New rule', deleted: false, createdAt: now()}, increments nextRuleId by 1
  #   8. User runs 'fspec remove-rule AUTH-001 2' twice, first call sets deleted=true, second call succeeds with message 'Item ID 2 already deleted' (idempotent)
  #   9. User runs 'fspec restore-rule AUTH-001 2' on non-deleted item, command succeeds with message 'Item ID 2 already active' (idempotent)
  #   10. User runs 'fspec restore-rule AUTH-001 99', command fails with error 'Item ID 99 does not exist' (validation)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we provide a 'show-deleted' command to view deleted items (with IDs and deletedAt timestamps) for debugging and recovery purposes?
  #   A: true
  #
  #   Q: Should compact-work-unit require confirmation prompt or run silently? Compaction is permanent and removes deleted items forever.
  #   A: true
  #
  #   Q: Should we warn or auto-compact when deleted item count exceeds a threshold (e.g., >50% of total items)? Or leave compaction entirely manual/on-done?
  #   A: true
  #
  #   Q: Should restore commands support bulk operations (e.g., 'fspec restore-rule AUTH-001 2,5,7' to restore multiple IDs at once)?
  #   A: true
  #
  #   Q: Should we provide a 'purge' command to permanently delete specific items by ID before compaction (for removing sensitive data immediately)?
  #   A: true
  #
  #   Q: Should display commands show creation/deletion timestamps in verbose mode (e.g., 'fspec show-work-unit AUTH-001 --verbose')?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No automatic threshold-based compaction. Only auto-compact on 'done' status. User has full control over when to compact via manual command
  #   2. No separate purge command. Use compact-work-unit for permanent deletion. Simpler API with fewer commands to maintain
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec for Example Mapping
    I want to remove multiple items from collections (rules, examples, questions, notes) without index shifts
    So that sequential removals target correct items and no data is lost due to array splicing

  Scenario: Sequential removal without index shifts
    Given work unit AUTH-001 has 5 rules with IDs [0, 1, 2, 3, 4]
    And all rules have deleted: false
    When AI removes rule at index 1
    Then rule[1].deleted should be set to true
    And rule[1].deletedAt should be set to ISO timestamp
    And rule indices should remain [0, 1, 2, 3, 4]
    When AI removes rule at index 2
    Then rule[2].deleted should be set to true
    And rule[2].deletedAt should be set to ISO timestamp
    And rule indices should remain [0, 1, 2, 3, 4]
    And no rules should be shifted or lost

  Scenario: Display with gaps in indices after deletions
    Given work unit AUTH-001 has 5 rules: [0] Rule A, [1] Rule B, [2] Rule C, [3] Rule D, [4] Rule E
    When AI removes rules at indices 1 and 3
    And I run "fspec show-work-unit AUTH-001"
    Then display should show rules:
      """
      [0] Rule A
      [2] Rule C
      [4] Rule E
      """
    And display should show "3 active items (2 deleted)"
    And indices [1] and [3] should not appear in display

  Scenario: Restore deleted item to original index
    Given work unit AUTH-001 has rule at index 2 with deleted: true
    And rule[2].deletedAt is "2025-01-31T12:00:00.000Z"
    When I run "fspec restore-rule AUTH-001 2"
    Then rule[2].deleted should be set to false
    And rule[2].deletedAt should be cleared (undefined)
    And rule should reappear in display at index [2]

  Scenario: Manual compaction removes deleted items and renumbers IDs
    Given work unit AUTH-001 has 10 rules with IDs [0-9]
    And 4 rules are deleted (IDs 1, 3, 5, 7)
    And nextRuleId is 10
    When I run "fspec compact-work-unit AUTH-001"
    Then system should prompt for confirmation
    When I confirm the operation
    Then deleted rules should be permanently removed
    And remaining 6 rules should be renumbered to IDs [0-5]
    And nextRuleId should be reset to 6
    And rules should be sorted by createdAt timestamp

  Scenario: Auto-compact on done status change
    Given work unit AUTH-001 is at "implementing" status
    And work unit has 10 rules with 3 deleted
    When I run "fspec update-work-unit-status AUTH-001 done"
    Then auto-compact should trigger before status change
    And deleted rules should be permanently removed
    And remaining rules should be renumbered sequentially
    And nextRuleId should be reset to remaining item count
    And then status should update to "done"

  Scenario: Migration converts string arrays to object arrays with stable IDs
    Given I have work-units.json at version "0.6.0"
    And work unit AUTH-001 has rules: ["Rule A", "Rule B"]
    And work unit has NO nextRuleId field
    When migration 001-stable-indices.ts runs
    Then rules should be converted to:
      """
      [
        {id: 0, text: "Rule A", deleted: false, createdAt: "2025-01-31T12:00:00.000Z"},
        {id: 1, text: "Rule B", deleted: false, createdAt: "2025-01-31T12:00:00.000Z"}
      ]
      """
    And nextRuleId should be set to 2
    And all collections should be migrated (rules, examples, questions, notes)

  Scenario: Add operation creates object with stable ID
    Given work unit AUTH-001 has nextRuleId: 5
    When I run "fspec add-rule AUTH-001 'New rule'"
    Then system should create rule object:
      """
      {
        id: 5,
        text: "New rule",
        deleted: false,
        createdAt: "2025-01-31T12:00:00.000Z"
      }
      """
    And nextRuleId should increment to 6

  Scenario: Idempotent remove operation on already-deleted item
    Given work unit AUTH-001 has rule at index 2 with deleted: true
    When I run "fspec remove-rule AUTH-001 2"
    Then command should succeed
    And output should show "Item ID 2 already deleted"
    And rule[2].deleted should remain true
    And no error should be thrown

  Scenario: Idempotent restore operation on already-active item
    Given work unit AUTH-001 has rule at index 2 with deleted: false
    When I run "fspec restore-rule AUTH-001 2"
    Then command should succeed
    And output should show "Item ID 2 already active"
    And rule[2].deleted should remain false
    And no error should be thrown

  Scenario: Restore validation fails for non-existent ID
    Given work unit AUTH-001 has rules with IDs [0-9]
    When I run "fspec restore-rule AUTH-001 99"
    Then command should fail with exit code 1
    And error output should contain "Item ID 99 does not exist"
    And no data should be modified

  Scenario: Show deleted items command displays soft-deleted items
    Given work unit AUTH-001 has 5 rules
    And rules at indices 1 and 3 are deleted
    When I run "fspec show-deleted AUTH-001"
    Then output should show deleted items:
      """
      [1] Rule B (deleted: 2025-01-31T12:00:00.000Z)
      [3] Rule D (deleted: 2025-01-31T13:30:00.000Z)
      """
    And output should show "2 deleted items"

  Scenario: Bulk restore with comma-separated IDs
    Given work unit AUTH-001 has rules with IDs [0-9]
    And rules at indices 2, 5, 7 are deleted
    When I run "fspec restore-rule AUTH-001 2,5,7"
    Then all IDs should be validated before restoring
    And rules[2], rules[5], rules[7] should all have deleted: false
    And deletedAt fields should be cleared for all three rules
    And output should show "Restored 3 items"

  Scenario: Verbose mode displays timestamps
    Given work unit AUTH-001 has 5 rules
    And rule[0] has createdAt: "2025-01-31T10:00:00.000Z"
    And rule[1] has deleted: true, deletedAt: "2025-01-31T12:00:00.000Z"
    When I run "fspec show-work-unit AUTH-001 --verbose"
    Then output should display createdAt for all items
    And output should display deletedAt for deleted items
    And timestamps should be in ISO 8601 format

  Scenario: Remove validation fails for non-existent ID
    Given work unit AUTH-001 has rules with IDs [0-9]
    When I run "fspec remove-rule AUTH-001 99"
    Then command should fail with exit code 1
    And error output should contain "Rule with ID 99 not found"
    And no data should be modified

  Scenario: Compact during specifying phase requires force flag
    Given work unit AUTH-001 is at "specifying" status
    And work unit has 5 rules with 2 deleted
    When I run "fspec compact-work-unit AUTH-001"
    Then command should fail with error message
    And error output should contain "Compaction permanently removes deleted items. Use --force to confirm."
    When I run "fspec compact-work-unit AUTH-001 --force"
    Then deleted rules should be permanently removed
    And remaining rules should be renumbered sequentially
    And output should show warning "⚠ Warning: Compacting during 'specifying' status permanently removes deleted items"
    And command should exit with code 0

  Scenario: Migration handles partially migrated data
    Given work unit AUTH-001 has mixed format rules:
      """
      [
        "Old string rule",
        { "id": 1, "text": "New object rule", "deleted": false, "createdAt": "2025-01-31T10:00:00.000Z" }
      ]
      """
    When migration 001-stable-indices.ts runs
    Then migration should detect mixed format
    And all rules should be normalized to object format:
      """
      [
        { "id": 0, "text": "Old string rule", "deleted": false, "createdAt": "2025-01-31T12:00:00.000Z" },
        { "id": 1, "text": "New object rule", "deleted": false, "createdAt": "2025-01-31T10:00:00.000Z" }
      ]
      """
    And nextRuleId should be set to 2
