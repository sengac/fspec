@done
@BUG-243
@phase-1
@persistence
@context-management
Feature: Preserve legacy provider state coordinates and continuation
  """
  Internal checkpoint positions have bounded versioned identity markers.
  Migration publishes a manifest reference without rewriting originals or
  changing any persisted coordinate. Uncommitted staging files are not state.
  """

  Background: User Story
    As an agent recovering an existing conversation
    I want stable original coordinates and verified native continuation
    So that migration preserves authentic evidence and lineage

  Scenario: Traverse legacy checkpoint coordinates without fabricated dialogue
    Given a legacy mixed manifest with a megabyte checkpoint lacking an envelope UUID
    When bounded original pages traverse its inclusive persisted range
    Then the checkpoint position returns an explicit bounded provider state identity
    And dialogue identities and continuation coordinates remain unchanged
    And neither the manifest nor stored messages are modified

  Scenario: Migrate legacy state idempotently across interrupted promotion
    Given a legacy mixed session and an interrupted checkpoint staging file
    When migration is repeated before and after a cold reload
    Then original bytes coordinates and native state identity remain unchanged
    And only verified committed state is selected for continuation
