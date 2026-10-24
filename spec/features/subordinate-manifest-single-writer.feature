@agent-core
@persistence
@agent-manager
@phase-1
Feature: Subordinate session creation has one manifest writer
  Spawned agents become available only after their session is durably created.

  """
  SessionManager owns initial manifest persistence after successful construction.
  AgentManager must not pre-create a manifest: doing so commits revision one and
  makes the manager's initial revision-zero snapshot stale. Both Rust and legacy
  NAPI handlers delegate persistence to SessionManager. This supersedes AMGR-014's
  pre-creation and degraded-persistence architecture without weakening revision
  conflict detection.
  """

  Scenario: Spawn persists one complete manifest and registers the subordinate
    Given a parent session using a Codex model
    When the parent spawns a subordinate with a reviewer role
    Then spawning succeeds and the subordinate is idle in the parent's manager
    And the subordinate has the reviewer role and the parent relationship
    And one initial committed manifest contains the full provider and model

  Scenario: Failed model selection leaves no orphan subordinate manifest
    Given a parent session whose spawn handler has an invalid model selection
    When the parent attempts to spawn a subordinate
    Then spawning reports the model selection failure
    And no subordinate session or manifest is created

  Scenario: Manifest persistence failure prevents subordinate registration
    Given a parent session whose session store cannot create manifest files
    When the parent attempts to spawn a subordinate
    Then spawning reports the persistence failure
    And no subordinate session or parent relationship is registered
