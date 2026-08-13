@model-selection
@wip
@ts-parity
@model-selector
@tui
@PROV-127
Feature: Model selector header count pluralization (TS parity)
  """
  PROV-127 pluralization rule. The shared helper
  rust/fspec-tui/src/views/model_selector/rows.rs::model_count_label renders
  "(1 model)" for a single model and "(N models)" otherwise (including
  "(0 models)"). It is used by the full-screen provider header rows (rows.rs)
  and the selector title (state.rs::title_text) so the singular / plural rule
  lives in exactly one place, matching the TypeScript reference labels.
  """

  Background: User Story
    As a developer using the /model selector
    I want provider header model counts to be grammatically correct
    So that a single-model provider reads "(1 model)" instead of "(1 models)"

  Scenario: A provider with exactly one model is labelled with the singular noun
    Given a provider header for a provider with exactly one model
    When the header label is rendered
    Then the label ends with "(1 model)"

  Scenario: A provider with two models is labelled with the plural noun
    Given a provider header for a provider with two models
    When the header label is rendered
    Then the label ends with "(2 models)"
