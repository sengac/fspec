@mermaid
@attachment
@validation
@validator
@critical
Feature: Mermaid Validation Fails with DOMPurify Error

  """
  Architecture notes:
  - Uses jsdom to create browser-like environment for Mermaid validation
  - DOMPurify must be available globally before Mermaid library initializes
  - Mermaid library internally calls DOMPurify.addHook during initialization
  - Solution requires ensuring DOMPurify is properly loaded in the jsdom window

  Critical implementation requirements:
  - MUST import and set up DOMPurify in jsdom window BEFORE importing mermaid
  - MUST use 'dompurify' package (not isomorphic-dompurify)
  - MUST call DOMPurify constructor with jsdom window: DOMPurify(window)
  - Global cleanup must handle DOMPurify references
  - Error handling should NOT expose DOMPurify errors to users
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mermaid validation must work in Node.js environment using jsdom
  #   2. DOMPurify must be properly initialized before Mermaid library loads
  #   3. Mermaid validation should fail gracefully with clear error messages
  #
  # EXAMPLES:
  #   1. User attaches architecture.md with valid Mermaid block, validation succeeds without DOMPurify error
  #   2. User attaches flowchart.mmd with valid syntax, no DOMPurify.addHook error occurs
  #   3. User attaches diagram.mermaid with invalid syntax, gets clear error message about syntax issue (not DOMPurify error)
  #
  # ========================================

  Background: User Story
    As a developer using fspec to attach Mermaid diagrams
    I want to validate Mermaid syntax without DOMPurify errors
    So that I can successfully attach diagram files to work units

  Scenario: Attach markdown file with valid Mermaid code block
    Given I have a work unit AUTH-001 in the backlog
    And I have a file "architecture.md" containing a mermaid code block with valid syntax
    When I run 'fspec add-attachment AUTH-001 architecture.md'
    Then the mermaid code block should be extracted and validated
    And the attachment should be added successfully
    And no DOMPurify error should occur

  Scenario: Attach Mermaid diagram file with valid syntax
    Given I have a work unit AUTH-001 in the backlog
    And I have a file "flowchart.mmd" with valid Mermaid syntax
    When I run 'fspec add-attachment AUTH-001 flowchart.mmd'
    Then the diagram should be validated successfully
    And no DOMPurify.addHook error should occur
    And the file should be copied to spec/attachments/AUTH-001/

  Scenario: Attach Mermaid diagram with invalid syntax shows clear error
    Given I have a work unit AUTH-001 in the backlog
    And I have a file "diagram.mermaid" with invalid Mermaid syntax
    When I run 'fspec add-attachment AUTH-001 diagram.mermaid'
    Then the command should fail with a clear syntax error message
    And the error should NOT mention DOMPurify
    And the error should indicate the syntax problem
