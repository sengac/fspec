@persistence
@session-management
@phase-1
@BUG-271
@critical
Feature: unreadable-session-isolation
  """
  Discover healthy sessions while retaining explicit unreadable-entry diagnostics. Keep unreadable manifests in place, targeted loads strict, and unknown-reference cleanup conservative. Surface storage issues without requiring an open session. See BUG-271 research attachment.
  """

  Background: User Story
    As a developer with existing session history
    I want unreadable historical sessions isolated
    So that I can start sessions without losing saved history

  Scenario: Empty historical manifests do not prevent new sessions
    Given a healthy session and an empty historical manifest
    When a fresh store creates a session with an explicit provider
    Then the new session is durable and the empty manifest is reported without changing its bytes

  Scenario: Unreadable sessions do not block another project
    Given a healthy project session and malformed history for another project
    When the user lists and resumes healthy history and starts another session
    Then healthy sessions remain available and the malformed file remains unchanged

  Scenario: Damaged target sessions fail explicitly
    Given truncated and incompatible historical manifests
    When the user loads or updates each damaged session
    Then each operation reports its path and cause without changing the damaged bytes

  Scenario: Supported legacy manifests remain readable without rewriting
    Given a valid older manifest missing defaultable fields
    When a fresh store reads the older session
    Then supported defaults are applied without rewriting the manifest

  Scenario: Unknown references prevent destructive cleanup
    Given an unreadable manifest and existing original messages
    When orphan cleanup inspects committed references
    Then cleanup fails conservatively and all original bytes remain intact

  Scenario: Store wide failures remain errors
    Given the sessions path is not a directory
    When a fresh store initializes
    Then initialization fails explicitly rather than reporting an empty usable store

  Scenario: Repair is discovered on the next cold load
    Given a healthy session and a damaged historical manifest across repeated starts
    When the damaged manifest is replaced with its valid backed up contents
    Then the next cold load discovers both sessions without rewriting healthy history
