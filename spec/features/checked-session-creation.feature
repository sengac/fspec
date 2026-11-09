@done
@cli
@session-management
@persistence
@phase-1
@BUG-271
@critical
Feature: Checked session creation
  """
  Checked creation retains application failure causes while the genuine missing-model decline is its own error naming the missing default model (BUG-410). This unchanged scenario is separated from session-storage-diagnostics to satisfy one test file per feature; the original feature and coverage remain attached to BUG-271.
  """

  Background: User Story
    As a developer
    I want session creation failures distinguished from missing model selection
    So that persistence errors never publish a successful or empty session

  Scenario: Checked creation distinguishes declines from application failures
    Given a fresh session manager with an offline model catalog
    When checked creation is attempted before model selection and after the sessions path becomes unusable
    Then the missing model returns a no-default-model error while the storage failure returns its own cause and publishes no session
