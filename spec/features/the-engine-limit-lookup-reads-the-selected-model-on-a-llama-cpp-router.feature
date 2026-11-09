@done
@model-selection
@rust
@providers
@BUG-562
Feature: The engine limit lookup reads the selected model on a llama.cpp router
  """
  Change in rust/providers/src/engine_limits.rs: context_window_report adds the model and autoload=false query for LocalEngine::LlamaCpp only, and fetch reports a non-success status with the server's error message (error.message or a plain error string). The recorded source string is unchanged. Tests: rust/providers/tests/bug562_llama_cpp_router_engine_limit.rs drives context_window_report against a wiremock router.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On llama.cpp the engine limit lookup asks GET /props with the query parameters model=<the selected model id> and autoload=false, the same rule as BUG-512: a single-model server ignores both, and a router answers from that model's own server (llama.cpp server-models.cpp:2021-2064, spec/attachments/BUG-512)
  #   2. The lookup never makes a router load a model (autoload=false); a router that has not loaded or does not know the model answers an error, and the lookup fails with the server's own error message, so BUG-547's flow takes over (the TUI asks for the limit)
  #   3. Strata keeps the bare GET /props (it is not a llama.cpp router); the source recorded for the value stays llama.cpp GET /props default_generation_settings.n_ctx
  #
  # EXAMPLES:
  #   1. A router serves gemma-4 (n_ctx 32768) and qwen3; bare GET /props answers the router's own props with n_ctx 0; a llama.cpp profile model gemma-4 with no contextWindow gets 32768 from GET /props?model=gemma-4&autoload=false
  #   2. The router has not loaded gemma-4 and answers 400 {error: {message: "model is not loaded"}}: the lookup fails naming llama.cpp GET /props and model is not loaded, and nothing is loaded
  #   3. A single-model llama.cpp server answers GET /props with n_ctx 65536 whatever the query: 65536 is used as before
  #
  # ========================================
  Background: User Story
    As a user running several local models behind one llama.cpp router
    I want to have fspec read the selected model's own running context size when the profile declares none
    So that selecting a router model works without being asked for a limit the server already knows

  Scenario: On a router the context window comes from the selected model's own props
    Given a llama.cpp router whose bare GET /props answers the router's own props with n_ctx 0
    And the router answers GET /props for model "gemma-4" without autoload with n_ctx 32768
    When the running context window of "gemma-4" is asked for
    Then it is 32768 from llama.cpp GET /props default_generation_settings.n_ctx

  Scenario: A router that has not loaded the model fails the lookup with its own reason
    Given a llama.cpp router that answers GET /props for model "gemma-4" without autoload with HTTP 400 and the error message "model is not loaded"
    When the running context window of "gemma-4" is asked for
    Then the lookup fails naming llama.cpp GET /props and "model is not loaded"
    And the router was never asked to load the model

  Scenario: A single-model server is asked as before
    Given a single-model llama.cpp server that answers GET /props with n_ctx 65536 whatever the query
    When the running context window of "gemma-4" is asked for
    Then it is 65536
