@done
@providers
@model-selection
@rust
@PROV-173
Feature: models.dev model limits have no guessed defaults
  """
  codelet_providers::models::LimitInfo mirrors the models.dev limit block
  ({context, output} as u32). PROV-173 removed its Default impl (a guessed
  128000/4096 any caller could reach). The cached payload has a limit block
  on every model, with 0 for limits that do not apply (159 contexts and 260
  outputs of 8932 models, mostly embedding and image models), so
  LimitInfo::context_window() and LimitInfo::max_output_tokens() read 0 as
  unavailable (None) and every consumer uses them instead of its own
  `> 0` filter. Tests: rust/providers/tests/prov173_models_dev_limits.rs.
  """

  Background: User Story
    As a user selecting models from the models.dev catalog
    I want a limit the catalog does not give to be reported as unavailable
    So that no model is budgeted against a guessed or zero limit

  Scenario: A zero catalog limit is unavailable
    Given a models.dev model entry whose limit is context 32768 and output 0
    When its limits are read
    Then its context window is 32768
    And its max output tokens are unavailable

  Scenario: Model limits have no guessed default
    Given the models.dev LimitInfo type source
    When its trait implementations are listed
    Then it does not implement Default
