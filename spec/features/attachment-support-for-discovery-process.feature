@done
@example-mapping
@file-ops
@discovery
@phase1
@FEAT-013
Feature: Attachment support for discovery process
  """
  Attachments array added to WorkUnit interface in src/types/index.ts
  Three new commands: add-attachment, list-attachments, remove-attachment
  Uses fs/promises for file operations (copyFile, stat, unlink)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a AI agent or developer using fspec
  #   I want to attach files to work units during Example Mapping
  #   So that I can supplement architecture notes and NFRs with diagrams, mockups, and documents
  #
  # BUSINESS RULES:
  #   1. Attachments must be stored in spec/attachments/<work-unit-id>/ directory
  #   2. Attachment paths must be stored as relative paths from project root
  #   3. Source file must exist before copying to attachments directory
  #
  # EXAMPLES:
  #   1. User runs 'fspec add-attachment AUTH-001 diagrams/auth-flow.png', file is copied to spec/attachments/AUTH-001/ and tracked in work unit
  #   2. User runs 'fspec list-attachments AUTH-001', sees all attached files with sizes and modification dates
  #   3. User runs 'fspec remove-attachment AUTH-001 diagram.png', file is deleted and tracking entry removed
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec
    I want to attach files to work units during Example Mapping
    So that I can supplement architecture notes and NFRs with diagrams, mockups, and documents

  Scenario: Add attachment to work unit
    Given I have a work unit "AUTH-001" in specifying status
    And I have a file "diagrams/auth-flow.png" in the current directory
    When I run "fspec add-attachment AUTH-001 diagrams/auth-flow.png"
    Then the file should be copied to "spec/attachments/AUTH-001/auth-flow.png"
    And the work unit should track the attachment path "spec/attachments/AUTH-001/auth-flow.png"
    And the output should display "✓ Attachment added successfully"

  Scenario: Add attachment with description
    Given I have a work unit "UI-002" in specifying status
    And I have a file "mockups/dashboard.png" in the current directory
    When I run "fspec add-attachment UI-002 mockups/dashboard.png --description 'Dashboard v2'"
    Then the file should be copied to "spec/attachments/UI-002/dashboard.png"
    And the output should display the description "Dashboard v2"

  Scenario: List attachments for work unit
    Given I have a work unit "AUTH-001" with attached files
    And the attachment "spec/attachments/AUTH-001/auth-flow.png" exists
    When I run "fspec list-attachments AUTH-001"
    Then the output should display the attachment path "spec/attachments/AUTH-001/auth-flow.png"
    And the output should display the file size
    And the output should display the modification date

  Scenario: List attachments shows no attachments
    Given I have a work unit "AUTH-002" with no attachments
    When I run "fspec list-attachments AUTH-002"
    Then the output should display "No attachments found for work unit AUTH-002"

  Scenario: Remove attachment from work unit
    Given I have a work unit "AUTH-001" with an attached file "diagram.png"
    When I run "fspec remove-attachment AUTH-001 diagram.png"
    Then the file "spec/attachments/AUTH-001/diagram.png" should be deleted
    And the work unit should no longer track the attachment
    And the output should display "✓ Attachment removed from work unit and file deleted"

  Scenario: Remove attachment but keep file on disk
    Given I have a work unit "AUTH-001" with an attached file "important-doc.pdf"
    When I run "fspec remove-attachment AUTH-001 important-doc.pdf --keep-file"
    Then the file "spec/attachments/AUTH-001/important-doc.pdf" should still exist
    And the work unit should no longer track the attachment
    And the output should display "✓ Attachment removed from work unit (file kept)"

  Scenario: Show work unit displays attachments
    Given I have a work unit "AUTH-001" with attached files
    When I run "fspec show-work-unit AUTH-001"
    Then the output should include an "Attachments:" section
    And the section should list all attachment paths

  Scenario: Attempt to add non-existent file
    Given I have a work unit "AUTH-001" in specifying status
    And the file "missing-file.png" does not exist
    When I run "fspec add-attachment AUTH-001 missing-file.png"
    Then the command should exit with code 1
    And the output should display "Error: Source file 'missing-file.png' does not exist"

  Scenario: Attempt to add attachment to non-existent work unit
    Given I have a file "diagram.png" in the current directory
    And the work unit "FAKE-001" does not exist
    When I run "fspec add-attachment FAKE-001 diagram.png"
    Then the command should exit with code 1
    And the output should display "Error: Work unit 'FAKE-001' does not exist"
