@done
@file-ops
@attachment-management
@bug-fix
@high
@BUG-151
Feature: add-attachment truncates the source file to 0 bytes when it already lives in spec/attachments/<ID>/
  """
  Implemented in src/commands/add-attachment.ts (addAttachment). Fix for BUG-151: canonicalize source and destination via path.resolve plus fs.realpath (defeats symlink aliasing); when they are equal, register-only (no copyFile, no unlink). The duplicate-registration check runs BEFORE any filesystem mutation. BUG-055 root-directory move behavior (copy into per-work-unit dir, then unlink root copy) is preserved and never triggered on the register-only path. Work unit registration persisted via fileManager.transaction (LOCK-002).
  """

  Background: User Story
    As a developer using fspec to manage work unit attachments
    I want to register a file that already lives in spec/attachments/<workUnitId>/ as an attachment without the file being destroyed
    So that I can safely write research docs directly into the attachments directory and then register them

  Scenario: Register a file that already lives in the work unit attachments directory
    Given I have a work unit "TEST-001"
    And a file "spec/attachments/TEST-001/notes.md" with content "important research"
    When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
    Then the command should succeed
    And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment

  Scenario: Duplicate registration is rejected without touching the file
    Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
    When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001" again
    Then the command should fail with an "already exists" error
    And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"

  Scenario: Register a read-only file already in the attachments directory without attempting a copy
    Given I have a work unit "TEST-001"
    And a read-only file "spec/attachments/TEST-001/notes.md" with content "important research"
    When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
    Then the command should succeed
    And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment

  Scenario: Duplicate registration from a different source file does not overwrite the registered attachment
    Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
    And a file "other/notes.md" with content "different content"
    When I add the attachment "other/notes.md" to work unit "TEST-001"
    Then the command should fail with an "already exists" error
    And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"

  Scenario: File in the attachments root is still moved into the work unit directory
    Given I have a work unit "TEST-001"
    And a file "spec/attachments/analysis.md" with content "root analysis"
    When I add the attachment "spec/attachments/analysis.md" to work unit "TEST-001"
    Then the file should exist at "spec/attachments/TEST-001/analysis.md" with content "root analysis"
    And the file "spec/attachments/analysis.md" should no longer exist
    And the work unit should track "spec/attachments/TEST-001/analysis.md" as an attachment

  Scenario: Symlink alias of the destination file does not truncate it
    Given I have a work unit "TEST-001"
    And a file "spec/attachments/TEST-001/notes.md" with content "important research"
    And a symlink outside the attachments directory pointing at "spec/attachments/TEST-001/notes.md"
    When I add the attachment via the symlink path to work unit "TEST-001"
    Then the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
