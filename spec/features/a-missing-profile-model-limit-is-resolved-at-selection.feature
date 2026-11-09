@BUG-582
@done
@model-selection
@rust
@providers
@critical
@BUG-547
Feature: A profile model with no max output limit fails at the first turn with a vague provider error instead of at selection
  """
  Providers: engine_limits asks the profile's declared engine for its running context window (vLLM/SGLang /v1/models max_model_len, llama.cpp/Strata /props n_ctx, LM Studio /api/v1/models loaded context_length, TGI /info max_total_tokens, Ollama /api/ps context_length); TensorRT-LLM, LMDeploy and profiles without an engine are not asked, and no engine reports a max output. The reported value is kept on the ProviderManager keyed by the selection and ranks below every declared limit.
  Sessions: resolve_provider_manager_and_limits (session create, resume and model switch) asks the engine for a missing context window, then refuses a profile model still missing a limit with MissingProfileLimit, whose message names the field, the profile, the fspec-config.json path and the TUI provider settings and can be parsed back.
  TUI: a model selection or session creation refused with that message opens a number prompt; Enter saves the value to the profile (save_profile with the full current definition) and retries; Esc shows the refusal. Provider settings label a profile missing contextWindow or maxOutputTokens.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A local profile model's context window and max output come from what is declared: the profile's contextWindow / maxOutputTokens, or its customModels entry's, which replace the profile's; a declared limit always wins (PROV-165; product owner, 2026-11-06)
  #   2. When a local profile model has no declared context window or max output, fspec asks the profile's engine where that engine reports it over HTTP (fields confirmed from engine source, research attachment) and uses the reported value, recording where it came from (engine, endpoint, field); nothing is guessed or defaulted (product owner, 2026-11-06)
  #   3. If the engine reports nothing usable, the TUI asks the user for the missing value when the model is selected, saves it to the profile in fspec-config.json and continues the selection (product owner, 2026-11-06)
  #   4. Only if the user cancels is the model refused, with a message naming the missing field, the profile and where to set it: the fspec-config.json path (providers.openai.profiles.<name>.maxOutputTokens or contextWindow) and the TUI provider settings (product owner, 2026-11-06)
  #   5. The check runs at selection or session creation (session create, resume and model switch), never first at a turn; a session is not created and a model switch does not happen with a missing limit (product owner, 2026-11-06)
  #   6. The TUI provider settings show a profile that is missing a limit (product owner, 2026-11-06)
  #   7. No default is ever filled in: an engine value of 0, -1, null or a value that is not a whole positive token count is not usable (no silent defaults)
  #   8. Engine reports used (research attachment, from source): the running context window from vLLM and SGLang GET /v1/models data[].max_model_len, llama.cpp and Strata GET /props default_generation_settings.n_ctx, LM Studio GET /api/v1/models loaded_instances[].config.context_length, TGI GET /info max_total_tokens, and Ollama GET /api/ps models[].context_length (loaded models only). Training-context values (Ollama /api/show, llama.cpp n_ctx_train) are not used. No engine reports a max output cap, so a missing max output always goes to the user; TensorRT-LLM and LMDeploy report nothing
  #   9. The engine is the profile's declared engine (MODEL-016); a profile without an engine is not probed (the engine is never inferred) and goes straight to the user prompt
  #
  # EXAMPLES:
  #   1. The Strata profile declares contextWindow 200000 and no maxOutputTokens, and Strata reports no max output: selecting openai:Strata/qwen3.8-flash-next-iq3_s in the TUI asks for Max Output Tokens; the user enters 10000, it is saved as providers.openai.profiles.Strata.maxOutputTokens, the session is created and the first turn runs
  #   2. Same, but the user cancels: the selection is refused with 'openai:Strata/qwen3.8-flash-next-iq3_s has no max output tokens: set maxOutputTokens on the Strata profile in ~/.fspec/fspec-config.json (providers.openai.profiles.Strata) or in the TUI provider settings'; no session is created
  #   3. A llama.cpp profile declares no contextWindow and GET /props reports the running context size: that value is used, and the session records that the context window came from llama.cpp GET /props
  #   4. A vLLM profile declares contextWindow 32768 and GET /v1/models reports max_model_len 131072: the declared 32768 is used
  #   5. A profile declares both limits: selection never calls the engine and works as today
  #   6. The TUI provider settings list the Strata profile with a note that it has no max output tokens
  #   7. A TensorRT-LLM profile declares no contextWindow: the engine reports nothing, so the TUI asks for Context Window
  #
  # ========================================
  Background: User Story
    As a user running a local inference server profile\nI want a model's missing context window or max output resolved from the engine or from me when I select it\nSo that a model never fails at its first turn with a vague provider error and no limit is made up

  Scenario: A declared limit always wins over the engine
    Given a vLLM profile declares contextWindow 32768 and maxOutputTokens 4096 and the engine reports max_model_len 131072
    When a session is created on its model
    Then the session's context window is 32768 and its max output is 4096
    And the engine is not asked

  Scenario: A context window the engine reports is used and its source recorded
    Given a llama.cpp profile declares maxOutputTokens 4096 and no contextWindow
    And GET /props reports default_generation_settings.n_ctx 65536
    When a session is created on its model
    Then the session's context window is 65536
    And the context window is recorded as reported by llama.cpp GET /props

  Scenario: A missing limit is refused at selection, not at the first turn
    Given the Strata profile declares contextWindow 200000 and no maxOutputTokens, and no engine reports a max output
    When a session is created on openai:Strata/qwen3.8-flash-next-iq3_s without the TUI
    Then creation is refused naming maxOutputTokens, the Strata profile, the fspec-config.json path and the TUI provider settings
    And no session is created

  Scenario: The TUI asks for a missing limit, saves it and continues
    Given the Strata profile declares contextWindow 200000 and no maxOutputTokens
    When the user selects openai:Strata/qwen3.8-flash-next-iq3_s in the TUI and enters 10000 for Max Output Tokens
    Then providers.openai.profiles.Strata.maxOutputTokens is 10000 in fspec-config.json
    And the session is created and the provider its first turn uses caps output at 10000

  Scenario: Cancelling the prompt refuses the model
    Given the Strata profile declares contextWindow 200000 and no maxOutputTokens
    When the user selects openai:Strata/qwen3.8-flash-next-iq3_s in the TUI and cancels the prompt
    Then the selection is refused with the message naming maxOutputTokens, the Strata profile and where to set it
    And fspec-config.json is unchanged

  Scenario: An engine that reports nothing usable goes to the user
    Given a TensorRT-LLM profile declares maxOutputTokens 4096 and no contextWindow
    When a session is created on its model without the TUI
    Then creation is refused naming contextWindow and the profile

  Scenario: The provider settings show a profile missing a limit
    Given the Strata profile declares no maxOutputTokens
    When the TUI provider settings list the profiles
    Then the Strata profile is shown as missing max output tokens
