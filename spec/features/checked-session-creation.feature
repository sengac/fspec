@done
@cli
@session-management
@persistence
@phase-1
@BUG-271
@critical
Feature: Checked session creation
  """
  Checked creation retains application failure causes while preserving the genuine missing-model decline. This unchanged scenario is separated from session-storage-diagnostics to satisfy one test file per feature; the original feature and coverage remain attached to BUG-271.
  """

  Background: User Story
    As a developer
    I want session creation failures distinguished from missing model selection
    So that persistence errors never publish a successful or empty session

  Scenario: Checked creation distinguishes declines from application failures
    Given a fresh session manager with an offline model catalog
    When checked creation is attempted before model selection and after the sessions path becomes unusable
    Then only the missing model returns an empty decline while the storage failure returns its cause and publishes no session
