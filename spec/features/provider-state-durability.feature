@done
@BUG-243
@phase-1
@persistence
@context-management
Feature: Acknowledge provider checkpoints durably
  """
  An immutable checkpoint root is written and synced before atomic publication.
  Its directory is acknowledged before the manifest reference is promoted.
  A failed manifest directory acknowledgement reports promoted state explicitly.
  """

  Background: User Story
    As an agent publishing provider continuation
    I want ordered durability barriers and idempotent retry
    So that acknowledgement reflects the committed checkpoint

  Scenario: Acknowledge checkpoint state only after ordered durability barriers
    Given a real provider checkpoint writer with an existing native checkpoint
    When checkpoint publication reaches each durable promotion boundary
    Then data sync precedes atomic replacement and directory acknowledgement
    And failures before replacement preserve the previous checkpoint
    And failure after replacement reports promoted state explicitly and retry is idempotent
