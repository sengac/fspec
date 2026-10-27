@done
@bug-fix
@phase-1
@BUG-273
@critical @persistence @session-management
Feature: Responsive session search

  """
  Blocking handlers and result serialization run on the existing Tokio blocking pool. Hook denials remain before dispatch; no new runtime or live storage access.
  """

  Background: User Story
    As a developer
    I want search to wait outside async workers
    So that unrelated work remains responsive


  Scenario: Search does not block async workers
    Given a SessionSearch handler waiting for storage
    When the tool runs on a single async worker
    Then unrelated async work continues and the search result is preserved
