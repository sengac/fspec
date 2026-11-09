@done
@REFAC-029
@MODEL-001
@providers
@provider
@model-selection
@rust
Feature: Model registry selection and filtering
  """
  Split from dynamic-model-selection-via-models-dev (REFAC-029) so each
  feature has one test file under the 300-line limit: that feature keeps the
  models.dev cache scenarios (rust/providers/tests/model_selection_test.rs);
  this one holds ModelRegistry selection, listing, filtering and search
  (rust/providers/tests/model_registry_selection_test.rs). Both share the
  fixture in rust/providers/tests/model_selection_support/mod.rs.
  BUG-335 removed prefix matching from ModelRegistry::get_model; the stale
  comments claiming "claud" matches by prefix are gone, the unknown-model
  scenario uses "claud" itself, and a prefix of a catalog key is not found.
  rust/providers/src/models/registry.rs moves its unit tests to
  registry/tests.rs to stay under 300 lines.
  """

  Background: User Story
    As a developer using codelet
    I want to select, list and filter models from the models.dev registry
    So that I can choose the best model for my task without hardcoded limitations

  Scenario: Select model using full provider/model path
    Given the models cache contains anthropic provider with claude-sonnet-4 model
    When I run codelet with --model anthropic/claude-sonnet-4
    Then the model should be selected from the registry
    And the appropriate provider facade should be used

  Scenario: List models for a specific provider
    Given the models cache contains anthropic provider with multiple models
    When I run codelet models anthropic
    Then all Anthropic models should be displayed
    And each model should show name and capabilities

  Scenario: Unknown model shows error with fuzzy suggestions
    Given the models cache contains anthropic provider with claude models
    When I run codelet with --model anthropic/claud
    Then an error should be displayed: "Model 'claud' not found in provider 'anthropic'"
    And fuzzy match suggestions should be shown: claude-sonnet-4, claude-opus-4, claude-haiku

  Scenario: Reject model without tool_call capability
    Given the cache contains google/gemini-2.0-flash-thinking-exp with tool_call=false
    When I run codelet with --model google/gemini-2.0-flash-thinking-exp
    Then an error should be displayed: "Model google/gemini-2.0-flash-thinking-exp does not support tool_call"
    And the command should exit with code 1

  Scenario: Filter models by reasoning capability
    Given the cache contains models with and without reasoning capability
    When I run codelet models --reasoning
    Then only models with reasoning=true should be displayed

  Scenario: List all available providers
    Given the models cache contains anthropic, google, and openai providers
    When I run codelet models --providers
    Then a list of providers should be displayed
    And each provider should show id, name, and model count

  Scenario: Filter models by vision capability
    Given the cache contains models with and without vision capability
    When I run codelet models --vision
    Then only models with image input modality should be displayed

  Scenario: Search models with fuzzy matching
    Given the cache contains multiple models from various providers
    When I run codelet models --search "claude"
    Then all models containing "claude" in name or id should be displayed
    And results should include anthropic/claude-sonnet-4 and anthropic/claude-opus-4

  Scenario: Show verbose model details
    Given the cache contains anthropic/claude-sonnet-4 model
    When I run codelet models anthropic/claude-sonnet-4 --verbose
    Then detailed model information should be displayed
    And the output should include context limit, output limit, capabilities, and cost

  Scenario: A prefix of a catalog model id is not found
    Given the models cache contains anthropic provider with claude-sonnet-4 model
    When the model "claude-sonnet" is looked up in provider "anthropic"
    Then the lookup fails with a model-not-found error

  Scenario: The registry and model selection sources stay within the file size limit
    Given the files rust/providers/src/models/registry.rs and rust/providers/tests/model_selection_test.rs
    When their line counts and comments are read
    Then each has at most 300 lines
    And neither claims that a model id matches by prefix
