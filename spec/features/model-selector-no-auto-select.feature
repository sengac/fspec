@done
@tui
@model-selection
@rust
@PROV-101
Feature: Model selector does not auto-select when no current model

  # ARCHITECTURE NOTES (PROV-101 #4/#5):
  # rows::first_selectable_or_zero is removed. ModelSelectorView gains a
  # has_active_selection flag. set_providers seeds the cursor ONLY when
  # index_of_model matches the current model; otherwise no selection is active
  # (no auto-snap to index 0). Enter is a no-op while there is no active
  # selection; explicit user navigation (arrows/Home/End/filter) activates it.
  Background: User Story
    As a fspec TUI user
    I want the model selector to show nothing selected when I have no current model
    So that pressing Enter never silently picks an arbitrary first model

  Scenario: model-selector does not auto-select the first row when no current model
    Given a model selector with no current model set
    When the model selector loads providers with selectable rows
    Then the model selector reports no active selection
    And pressing Enter emits no model-selected action
