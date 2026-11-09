@done
@model-selection
@rust
@thinking-level
@providers
@BUG-512
Feature: The llama.cpp live check reads the selected model on a router
  """
  Change is in rust/providers/src/local_reasoning/live_check.rs only: the llama.cpp branch of LiveCheck::reported adds the model and autoload=false query parameters (reqwest query, URL-encoded), and fetch reports a non-success status with the server's error message (error.message object form used by llama.cpp and LM Studio, or a plain error string as Ollama sends) so a router refusal names its reason. Endpoint name stays GET /props. Not changed here: engine_limits.rs (BUG-547) reads llama.cpp /props n_ctx without ?model= and has the same router-mode gap; reported to the supervisor.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On llama.cpp the live check asks GET /props with the query parameters model=<the selected model id> and autoload=false; a single-model server ignores both (server-context.cpp:5894), and a router proxies the request to that model's own server (server-models.cpp:2021-2064)
  #   2. The live check never makes a router load a model: it only reports (MODEL-016 rule 14), and a router load can evict other loaded models and outlasts the check's timeout, so it always sends autoload=false
  #   3. A router that has not loaded the model, or does not know it, answers with an error; the check is reported as one that could not be made, naming the model, GET /props and the server's own error message, and the declaration still applies (MODEL-016 rule 15)
  #   4. A response with no chat_template_caps.reasoning_effort_levels (for example the router's own props) is a check that could not be made, never a mismatch or an agreement
  #
  # EXAMPLES:
  #   1. A router serves gemma-4 and qwen3; gemma-4 declares efforts [none, low, high] and GET /props?model=gemma-4&autoload=false reports [low, high]: no mismatch is reported, while GET /props without the model would have answered the router's own props with no chat_template_caps
  #   2. On that router gemma-4 declares efforts [low, high, max] and its own server reports [low, medium, high]: a mismatch is reported naming gemma-4, low, high, max and low, medium, high
  #   3. The router has not loaded gemma-4 and answers 400 {error: {message: "model is not loaded"}}: a failed live check is reported naming gemma-4, GET /props and model is not loaded; the declared levels still apply and the router is not asked to load the model
  #   4. A single-model llama.cpp server answers GET /props?model=gemma-4&autoload=false with its own props as before (it ignores the query), so single-server checks are unchanged
  #
  # ========================================
  Background: User Story
    As a user running several models behind one llama.cpp router
    I want to have the live check read the selected model's own template capabilities
    So that a mismatch report is about the model I chose, or I am told the check could not be made

  Scenario: On a router the live check reads the selected model's own props
    Given a llama.cpp router serving "gemma-4" and "qwen3" whose bare GET /props answers the router's own props
    And a llama.cpp profile whose model "gemma-4" declares efforts "none, low, high"
    And the router answers GET /props for model "gemma-4" without autoload with reasoning_effort_levels "low, high"
    When the model's capability is resolved
    Then no mismatch is reported

  Scenario: A router model's own levels that differ from the declaration are reported
    Given a llama.cpp router whose model "gemma-4" reports reasoning_effort_levels "low, medium, high" on GET /props for that model
    And a llama.cpp profile whose model "gemma-4" declares efforts "low, high, max"
    When the model's capability is resolved
    Then a mismatch is reported naming "gemma-4", the declared "low, high, max" and the reported "low, medium, high"

  Scenario: A router that has not loaded the model is reported as a check that could not be made
    Given a llama.cpp router that answers GET /props for model "gemma-4" without autoload with HTTP 400 and the error message "model is not loaded"
    And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    When the model's capability is resolved
    Then a failed live check is reported naming "gemma-4", GET /props and "model is not loaded"
    And the levels offered are "low, high"
    And the router was never asked to load the model

  Scenario: Props without template capabilities are a check that could not be made
    Given a llama.cpp server whose GET /props answers the router's own props with no chat_template_caps
    And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    When the model's capability is resolved
    Then a failed live check is reported naming "gemma-4", GET /props and chat_template_caps.reasoning_effort_levels

  Scenario: A single-model server is checked as before
    Given a single-model llama.cpp server that answers GET /props with reasoning_effort_levels "low, high" whatever the query
    And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    When the model's capability is resolved
    Then no mismatch is reported
