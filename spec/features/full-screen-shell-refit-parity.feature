@model-selection
@done
@ts-parity
@rust
@model-selector
@tui
@RPC-337
Feature: Full-screen shell refit parity

  """
  Refit the existing scaffolds (ProviderSettingsView, ResumeSessionView) onto the shared shell. Migrating these views to the shell MUST preserve rendered output (snapshot parity). SearchHistoryView's refit is deferred to RPC-339 (its editable-query title needs a title-renderer generalization of the shell).
  """

  Background: User Story
    As a fspec TUI user
    I want existing full-screen views migrated onto the shared shell without visual change
    So that the refactor is safe and the UX is preserved

  Scenario: Migrated view preserves rendered output
    Given the Provider Settings view rendered with its pre-migration scaffold
    When the same view is rendered through the shared shell
    Then the rendered output is identical to the pre-migration snapshot
