@done
@cli
@feature-management
@BUG-271
@persistence
@session-management
@phase-1
@critical
Feature: Session storage diagnostics
  """
  Non-draining manifest issue snapshots cross RPC independently of sessions. Checked creation preserves application errors while retaining the legacy missing-model decline. Diagnostics use wrapped scrollable global dialogs and never write or repair history.
  """

  Background: User Story
    As a developer
    I want to see session storage problems without an open session
    So that I can recover history and distinguish storage failures from model selection

  Scenario: Startup warnings are visible without a session
    Given unreadable history exists and no session is open
    When each transport bootstraps the application
    Then a scrollable warning exposes every path cause and recovery instruction without changing history

  Scenario: Reconnect replays unresolved manifest warnings
    Given an unresolved manifest warning was dismissed
    When the application reconnects without an open session
    Then the warning is visible again from a non draining snapshot

  Scenario: Creation failures are not missing model declines
    Given a selected model and an unusable sessions directory
    When the user creates a session from the dialog work unit or model selection
    Then each transport shows the storage failure globally without publishing a session or claiming a missing model

  Scenario: Missing model selection remains an explicit decline
    Given no default model is selected
    When each transport attempts session creation
    Then the application requests model selection and creates no session

  Scenario: Healthy creation preserves roles alongside warnings
    Given a selected model and unreadable historical manifests
    When each transport creates a session with a role
    Then a durable session with that role exists and unresolved warnings remain available
