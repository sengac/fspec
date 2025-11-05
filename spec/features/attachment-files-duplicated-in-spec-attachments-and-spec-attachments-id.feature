@high
@file-ops
@attachment-management
@bug-fix
@BUG-055
Feature: Attachment files duplicated in spec/attachments/ and spec/attachments/[ID]/
  """
  Uses copyFile() from fs/promises to copy attachments. Bug occurs when source file is in spec/attachments/ root - file gets copied but original not deleted. Fix requires checking if source is in root and deleting after successful copy.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Attachments MUST be stored ONLY in spec/attachments/{work-unit-id}/ directory
  #   2. Attachments MUST NOT appear in spec/attachments/ root directory
  #   3. When copying attachments from spec/attachments/ root, the source file MUST be deleted after successful copy
  #
  # EXAMPLES:
  #   1. File created in spec/attachments/CONFIG-003-integration-gap-analysis.md, then fspec add-attachment called, results in duplication in both root and CONFIG-003/ subdirectory
  #   2. User adds attachment from external path (e.g., /tmp/diagram.png), file copied to spec/attachments/{work-unit-id}/diagram.png, original stays in /tmp (correct behavior)
  #   3. After fix, files in spec/attachments/ root that get attached should be moved (not copied) to work unit subdirectory
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to add attachments to work units
    So that files are stored in the correct location without duplication

  Scenario: Prevent duplication when adding file from spec/attachments/ root
    Given I have a work unit "TEST-001"
    And I have a file "spec/attachments/analysis.md" in the root attachments directory
    When I run "fspec add-attachment TEST-001 spec/attachments/analysis.md"
    Then the file should be moved to "spec/attachments/TEST-001/analysis.md"
    And the file "spec/attachments/analysis.md" should no longer exist in the root directory
    And the work unit should track "spec/attachments/TEST-001/analysis.md" as an attachment

  Scenario: Normal attachment from external path
    Given I have a work unit "TEST-002"
    And I have a file "/tmp/diagram.png" outside the attachments directory
    When I run "fspec add-attachment TEST-002 /tmp/diagram.png"
    Then the file should be copied to "spec/attachments/TEST-002/diagram.png"
    And the original file "/tmp/diagram.png" should still exist
    And the work unit should track "spec/attachments/TEST-002/diagram.png" as an attachment

  Scenario: Verify no duplication after fix
    Given I have a work unit "TEST-003"
    And I have a file "spec/attachments/document.pdf" in the root attachments directory
    When I run "fspec add-attachment TEST-003 spec/attachments/document.pdf"
    Then there should be exactly one copy of the file in "spec/attachments/TEST-003/document.pdf"
    And there should be zero files in the root "spec/attachments/" directory named "document.pdf"
