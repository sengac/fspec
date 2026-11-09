@BUG-582
@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-016
Feature: Local inference servers declare their thinking levels, including numeric efforts
  """
  Architecture notes:
  - Declarations live in the local-server profile (sessions profile_sections): LocalServerProfile gains an engine (vllm | sglang | strata | llama-cpp | ollama | lm-studio | tgi | tensorrt-llm | lmdeploy) and an optional profile-wide reasoning declaration; CustomModelDef gains a per-model declaration that replaces the profile-wide one. A declaration holds named efforts, a numeric effort range (whole numbers or decimals), a token budget range and a template toggle key (enable_thinking | thinking). Config fields: "engine" and "thinkingLevels" on the profile, "thinkingLevels" on a customModels entry; thinkingLevels = {efforts, numericEfforts {kind whole|decimal, min, max}, tokenBudget {min, max}, templateToggle}.
  - codelet-providers models::reasoning: ReasoningLevel gains a numeric-effort kind and an On/Off toggle kind (today Effort | Budget); ReasoningCapability gains the numeric range and the toggle key. The declaration resolves into it like models.dev reasoning_options (MODEL-009).
  - codelet-agent-loop thinking_config.rs as landed in MODEL-010 (83958288f): ThinkingTarget::for_provider_manager gives every profile model no capability and the level goes unchecked; effort_wire's LocalOpenAi arm sends reasoning_effort plus chat_template_kwargs {thinking, enable_thinking} = level != none; a token budget on LocalOpenAi is refused. MODEL-016 changes all three for local profiles: the capability comes from the declaration (none declared refuses the level), the form comes from a per-engine wire table, and a declared budget is sent in the engine's own field. Rhai custom providers keep MODEL-010's unchecked behaviour. The MODEL-010 scenario "A local server gets reasoning_effort and both template toggles" is superseded by this feature and must be updated when MODEL-016 is implemented.
  - The live check (llama.cpp GET /props chat_template_caps.reasoning_effort_levels, Ollama POST /api/show thinking, LM Studio GET /api/v1/models capabilities.reasoning) runs where the capability is resolved, only reports, and never changes what is sent. Async home: the first turn after the profile model is selected (agent_loop/routing.rs), where the turn target is resolved and the session can show the report; model selection is synchronous.
  - Session storage, the thinking dialog and /thinking hold only Off/Low/Medium/High until MODEL-011, so these scenarios exercise the capability and request-building layer with levels chosen directly.
  - Research: spec/attachments/MODEL-016/local-engine-reasoning-research.md (engine facts with file:line references); decisions: spec/attachments/MODEL-009/thinking-levels-research.md section 8 and the MODEL-016 answers (product owner, 2026-11-04).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A local-server profile names its engine, one of vLLM, SGLang, Strata, llama.cpp, Ollama, LM Studio, TGI, TensorRT-LLM or LMDeploy; reasoning declarations on a profile with no engine or an unknown engine are an error naming the profile and the value, and nothing is guessed from the base URL or model id
  #   2. Numeric efforts are in scope (product owner, 2026-11-04, decision 4): a declared numeric effort range is either whole numbers or decimals with a min and max, and a numeric effort is sent as a JSON number in reasoning_effort, never as a token budget
  #   3. A chosen level the declaration does not include is refused before the request is sent, naming the model and its declared levels; a numeric effort outside the declared range, or a decimal for a whole-number range, is refused the same way (MODEL-009 rules 4 and 5)
  #   4. fspec sends the chosen level unchanged and never remaps it (MODEL-009 rule 6), even on engines that remap internally (Strata sends high as xhigh, Ollama sends xhigh as max): the user declares the names the model's template honours
  #   5. A token budget is sent in the engine's own field: thinking_token_budget on vLLM and TensorRT-LLM, reasoning_budget_tokens on llama.cpp and Strata, custom_params.thinking_budget on SGLang
  #   6. fspec never sends a reasoning field the engine does not take: TensorRT-LLM refuses unknown request fields and unread chat_template_kwargs, while TGI, Ollama and LMDeploy silently drop unknown fields
  #   7. A declared token-budget model lets the user pick a numeric budget within the declared min..max; fspec invents no named levels or budget table for it (product owner, 2026-11-04, decision 9)
  #   8. A local-server profile declares a reasoning capability in config, either profile-wide or per customModels entry: a list of named efforts, a numeric effort range, a token budget range and a template toggle key (enable_thinking or thinking), in any combination; it resolves into the model's reasoning capability like models.dev reasoning_options (MODEL-009)
  #   9. A profile-wide declaration applies to every model of the profile, and a per-model declaration replaces it for that model; the two are not merged (product owner, 2026-11-04)
  #   10. A local profile model with no declaration, profile-wide or per model, has no reasoning control: a chosen level is refused before the request is sent, with an error naming the model and saying to declare levels in its profile. This replaces MODEL-010's interim local-openai form (reasoning_effort plus fixed chat_template_kwargs thinking/enable_thinking = level != none) (product owner, 2026-11-04)
  #   11. A model that declares only a template toggle offers the levels On and Off; On sends chat_template_kwargs {<key>: true} and Off sends {<key>: false}, with no reasoning_effort (product owner, 2026-11-04)
  #   12. A model that declares efforts and a toggle sends chat_template_kwargs {<key>: false} at effort none and {<key>: true} at any other effort, alongside reasoning_effort; only the declared key is sent, and a model with no declared toggle gets no chat_template_kwargs (unlike MODEL-010's fixed thinking and enable_thinking pair)
  #   13. A declared part the engine has no field for is an error for that model only, naming the profile, the model, the engine and the part; other models still load. Numeric efforts only on SGLang (decimals 0.0 to 0.99) and Strata (whole numbers 1 to 100); token budgets not on Ollama, LM Studio, LMDeploy or TGI; template toggles not on Ollama, LM Studio or TGI, and only enable_thinking on Strata; TGI takes no reasoning parameter at all
  #   14. Named and numeric efforts are sent as reasoning_effort on /v1/chat/completions on vLLM, SGLang, Strata, llama.cpp, Ollama, LM Studio, TensorRT-LLM and LMDeploy; LM Studio gets reasoning_effort although its docs do not list it (product owner, 2026-11-04)
  #   15. The config declaration is authoritative. Where the engine reports levels live (llama.cpp GET /props, Ollama POST /api/show, LM Studio GET /api/v1/models), fspec reads them when it resolves the model's capability and reports any mismatch, naming the model, the declared levels and the reported ones; the declaration still applies, and live data never stands in for a missing declaration (product owner, 2026-11-04)
  #   16. A live check that cannot be made (endpoint unreachable, or the model not reported) is reported naming the model and the endpoint, and the declaration still applies; it is never silently skipped (derived from the no-silent-defaults rule and the 2026-11-04 answer that the declaration is authoritative)
  #   17. llama.cpp's reported reasoning_effort_levels never include none (its probe tries only minimal, low, medium, high, xhigh and max, research §4), so a declared none is not counted as a mismatch against llama.cpp
  #   18. Live reports are compared with the declaration word for word, except that Ollama false and LM Studio off match a declared none or Off, and Ollama true and LM Studio on match On; the comparison only feeds the mismatch report and never changes what is sent (product owner, 2026-11-04)
  #   19. Off is the model's own declared off form, as in MODEL-010: reasoning_effort none when none is declared (with {<key>: false} if a toggle is declared), {<key>: false} alone when only a toggle is declared, and refused naming the model when neither is declared
  #   20. Relative to MODEL-010 as landed (83958288f, rust/agent-loop/src/thinking_config.rs): a local profile model's capability comes from its declaration instead of none-and-unchecked; local-openai's fixed reasoning_effort plus chat_template_kwargs {thinking, enable_thinking} form becomes the engine's own form; a token budget, refused on local-openai by MODEL-010, is sent in the engine's budget field; Rhai custom providers keep MODEL-010's unchecked behaviour
  #
  # EXAMPLES:
  #   1. A Strata profile model qwen3.8-flash-next declares efforts [none, low, medium, xhigh]; at xhigh fspec sends reasoning_effort "xhigh" and at none it sends reasoning_effort "none"
  #   2. On that Strata model, choosing high is refused before the request is sent, naming qwen3.8-flash-next and listing none, low, medium, xhigh; fspec does not send high for Strata to remap
  #   3. A Strata DeepSeek-V4.1 model declares efforts [none, low, medium, high, max] and whole-number efforts 1 to 100; at 40 fspec sends reasoning_effort 40 as a JSON number
  #   4. On that DeepSeek-V4.1 model, 150 is refused naming the model and the range 1 to 100, and 40.5 is refused as not a whole number; nothing is sent
  #   5. An SGLang model declares decimal efforts 0.0 to 0.99; at 0.5 fspec sends reasoning_effort 0.5
  #   6. A vLLM DeepSeek-V3.1 model declares efforts [none, high] and toggle thinking; at high fspec sends reasoning_effort "high" with chat_template_kwargs {thinking: true}, and at none it sends reasoning_effort "none" with chat_template_kwargs {thinking: false}
  #   7. A model declaring a token budget of 128 to 32768, at 4096, sends thinking_token_budget 4096 on vLLM, reasoning_budget_tokens 4096 on llama.cpp and custom_params {thinking_budget: 4096} on SGLang
  #   8. A TGI profile model declaring efforts [low, high] is an error naming the profile, the model and engine TGI, while a vLLM profile in the same config still loads its models
  #   9. An Ollama model declaring toggle enable_thinking is an error naming Ollama and the template toggle; an LMDeploy model declaring a token budget is an error naming LMDeploy and the token budget
  #   10. A profile with customModels declaring efforts but no engine is an error naming the profile; fspec does not infer the engine from the base URL
  #   11. A Strata profile declares efforts [none, low, medium, xhigh] profile-wide and its model qwen3.8-flash-next has no declaration of its own: at medium fspec sends reasoning_effort "medium"
  #   12. A llama.cpp profile declares efforts [low, high] profile-wide and its model gemma-4 declares efforts [none, low, medium, high, max]: gemma-4 offers none, low, medium, high, max, and the profile-wide list is not merged in
  #   13. A vLLM profile model qwen3-32b with no declaration, chosen at high, is refused before the request is sent with an error naming qwen3-32b and saying to declare its levels in the profile; neither reasoning_effort nor chat_template_kwargs is sent, where MODEL-010 sent reasoning_effort "high" with chat_template_kwargs {thinking: true, enable_thinking: true}
  #   14. A TensorRT-LLM model qwen3-32b declares only toggle enable_thinking: it offers On and Off; On sends chat_template_kwargs {enable_thinking: true} and Off sends {enable_thinking: false}, and no reasoning_effort is sent
  #   15. A vLLM Qwen3 model declares efforts [none, low, high] and no toggle: at low fspec sends reasoning_effort "low" and no chat_template_kwargs
  #   16. An LM Studio model declares efforts [low, medium, high]: at medium fspec sends reasoning_effort "medium" on /v1/chat/completions
  #   17. A llama.cpp model declares efforts [none, low, high, max] and GET /props reports reasoning_effort_levels [low, medium, high]: fspec reports a mismatch naming the model, the declared levels and the reported ones, and still offers none, low, high, max
  #   18. A llama.cpp model declares efforts [none, low, high] and GET /props reports [low, high]: no mismatch is reported, because llama.cpp never reports none
  #   19. A llama.cpp model with no declaration whose GET /props reports [low, high] still has no reasoning control, and a chosen level is refused with the declare-in-profile error
  #   20. An Ollama profile whose server cannot be reached for POST /api/show reports that the live check failed, naming the model and the endpoint, and the model keeps its declared levels
  #   21. A Strata model declaring toggle thinking is an error naming Strata and the toggle key, since Strata reads only enable_thinking; an LM Studio model declaring a token budget is an error naming LM Studio and the token budget
  #   22. An LMDeploy model declares efforts [low, high] and toggle enable_thinking: Off sends chat_template_kwargs {enable_thinking: false} and no reasoning_effort, where MODEL-010 refused Off for want of a none effort or sent reasoning_effort none, which LMDeploy rejects
  #   23. An Ollama model declares efforts [none, low, medium, high] and POST /api/show reports values [false, low, medium, high]: no mismatch is reported, because false matches the declared none
  #   24. An LM Studio model declares efforts [low, medium, high] and GET /api/v1/models reports allowed_options [off, on]: fspec reports a mismatch naming the model, the declared low, medium, high and the reported off, on, and still sends reasoning_effort at the declared levels
  #
  # QUESTIONS (ANSWERED):
  #   Q: Live discovery. Three engines report per-model levels at runtime: llama.cpp GET /props (chat_template_caps.reasoning_effort_levels, found by probing the template), Ollama POST /api/show (thinking {values, default}) and, per its docs, LM Studio GET /api/v1/models (capabilities.reasoning {allowed_options off|on|low|medium|high, default}). vLLM, SGLang, Strata, TGI, TensorRT-LLM and LMDeploy report nothing. Should MODEL-016 (a) use config declarations only, (b) also read these endpoints and use them when a model has no declaration, or (c) read them and report a mismatch with a declaration? (MODEL-009 research §7.1.2 left this as a DECISION.)
  #   A: Config plus live check (product owner, 2026-11-04). The config declaration is authoritative. fspec also reads the engine's live endpoint where one exists (llama.cpp GET /props, Ollama POST /api/show, LM Studio GET /api/v1/models) and reports any mismatch with the declaration; it never uses live data in place of a declaration.
  #
  #   Q: Declaration scope. Should a profile be able to declare reasoning levels once for every model it serves (Strata and llama.cpp usually serve one model), or only per customModels entry? If both are allowed, which one wins when they differ? (MODEL-009 research §7.1.2 left this as a DECISION.)
  #   A: Both levels are allowed and the model wins (product owner, 2026-11-04): a profile-wide declaration applies to all of the profile's models, and a per-model declaration replaces it for that model.
  #
  #   Q: LM Studio. Its server has no public source. Its docs list no reasoning_effort, chat_template_kwargs or budget for /v1/chat/completions, which is the path fspec's local profiles use. They document reasoning.effort only on /v1/responses for gpt-oss (0.3.29), and the native POST /api/v1/chat takes reasoning off|low|medium|high|on. Only a third-party page claims /v1/chat/completions honours reasoning_effort. Should MODEL-016 (a) give LM Studio profiles no reasoning control on the OpenAI path, (b) send reasoning_effort even though LM Studio does not document it, or (c) leave LM Studio to a later card that uses its native API?
  #   A: Send reasoning_effort on /v1/chat/completions for LM Studio (product owner, 2026-11-04).
  #
  #   Q: A local profile model with no reasoning declaration. That is every profile today, including the Strata profile in use, which gets reasoning_effort since BUG-440. MODEL-009 rule 3 gives a model with no capability no reasoning control. MODEL-010 question 0 asks whether to send the level unchecked instead. Which applies to undeclared profile models after MODEL-016: no reasoning control, or the level sent unchecked through the engine's form? This is the same decision as MODEL-010 question 0, so one answer can settle both cards.
  #   A: No reasoning control (product owner, 2026-11-04): a chosen level for an undeclared local profile model is refused before sending, with an error naming the model and saying to declare levels in its profile. This replaces MODEL-010's interim behaviour for local profiles.
  #
  #   Q: Toggle-only local models. Some models are switched only by a template toggle: Qwen3 on TensorRT-LLM, whose generic path ignores reasoning_effort, and Qwen3 on LMDeploy, which refuses reasoning_effort none. MODEL-009's level is a named effort or a token budget, so there is no on/off level, and the session holds only Off/Low/Medium/High until MODEL-011. How should the user turn such a model on or off: (a) a new on/off level kind added in MODEL-016, (b) Off sends {toggle: false} and Low/Medium/High send {toggle: true} until MODEL-011, or (c) toggle-only models get no control until MODEL-011? This overlaps MODEL-010 question 1 (toggle-only zai).
  #   A: Add an on/off level kind (product owner, 2026-11-04): for a toggle-only model the user picks On or Off, and fspec sends it through the declared toggle key.
  #
  #   Q: Comparing live reports whose words differ from the declaration. Ollama /api/show reports values that mix booleans and names (e.g. [false, true] or [false, low, medium, high]), and LM Studio /api/v1/models reports off|on|low|medium|high, while declarations use reasoning_effort names and On/Off. Proposed: false/off match a declared none or Off, true/on match On, and other names compare literally. Is that right? Only the Ollama and LM Studio mismatch checks depend on it; the llama.cpp check (names only) does not.
  #   A: Yes, as proposed (product owner, 2026-11-04): false/off match a declared none or Off, true/on match On, and other names compare literally. This applies only to the mismatch report and never changes what is sent. Rule 19 (a live check that cannot run is reported with the model and endpoint, and the declaration still applies) is confirmed.
  #
  # ========================================
  Background: User Story
    As a fspec user running a self-hosted inference server (Strata, SGLang, vLLM, llama.cpp, Ollama, LM Studio, TGI, TensorRT-LLM or LMDeploy)
    I want to declare in my local-server profile which thinking levels each model takes, including numeric efforts and template toggles, and have fspec send them in the form my engine expects
    So that I can choose a thinking level for a local model and know it reaches the engine unchanged, or is refused before the request is sent, never silently dropped or remapped

  Scenario: A declared effort is sent to Strata unchanged
    Given a Strata profile whose model "qwen3.8-flash-next" declares efforts "none, low, medium, xhigh"
    When a turn is sent at effort "xhigh"
    Then the request carries reasoning_effort "xhigh"
    When a turn is sent at effort "none"
    Then the request carries reasoning_effort "none"

  Scenario: An effort the declaration does not list is refused, not left for the engine to remap
    Given a Strata profile whose model "qwen3.8-flash-next" declares efforts "none, low, medium, xhigh"
    When a turn is sent at effort "high"
    Then the turn is refused before the request is sent
    And the error names "qwen3.8-flash-next" and lists "none, low, medium, xhigh"

  Scenario: A whole-number effort is sent to Strata as a JSON number
    Given a Strata profile whose DeepSeek-V4.1 model declares efforts "none, low, medium, high, max" and whole-number efforts from 1 to 100
    When a turn is sent at numeric effort 40
    Then the request carries reasoning_effort as the JSON number 40

  Scenario: A numeric effort outside the declared range or of the wrong kind is refused
    Given a Strata profile whose DeepSeek-V4.1 model declares whole-number efforts from 1 to 100
    When a turn is sent at numeric effort 150
    Then the turn is refused before the request is sent, naming the model and the range 1 to 100
    When a turn is sent at numeric effort 40.5
    Then the turn is refused before the request is sent, saying the model takes whole numbers

  Scenario: A decimal effort is sent to SGLang as a JSON number
    Given an SGLang profile whose model declares decimal efforts from 0.0 to 0.99
    When a turn is sent at numeric effort 0.5
    Then the request carries reasoning_effort as the JSON number 0.5

  Scenario: A declared template toggle follows the chosen effort
    Given a vLLM profile whose DeepSeek-V3.1 model declares efforts "none, high" and the template toggle "thinking"
    When a turn is sent at effort "high"
    Then the request carries reasoning_effort "high" and chat_template_kwargs thinking true
    And chat_template_kwargs has no enable_thinking key
    When a turn is sent at effort "none"
    Then the request carries reasoning_effort "none" and chat_template_kwargs thinking false

  Scenario: A model with no declared toggle gets no template kwargs
    Given a vLLM profile whose Qwen3 model declares efforts "none, low, high" and no template toggle
    When a turn is sent at effort "low"
    Then the request carries reasoning_effort "low"
    And the request carries no chat_template_kwargs, where MODEL-010 sent thinking and enable_thinking

  Scenario: A toggle-only model offers On and Off through its declared key
    Given a TensorRT-LLM profile whose model "qwen3-32b" declares only the template toggle "enable_thinking"
    Then the levels offered for "qwen3-32b" are "On, Off"
    When a turn is sent at On
    Then the request carries chat_template_kwargs enable_thinking true and no reasoning_effort
    When a turn is sent at Off
    Then the request carries chat_template_kwargs enable_thinking false and no reasoning_effort

  Scenario: Off uses the declared toggle when the model declares no none effort
    Given an LMDeploy profile whose model declares efforts "low, high" and the template toggle "enable_thinking"
    When a turn is sent at Off
    Then the request carries chat_template_kwargs enable_thinking false and no reasoning_effort
    And it is not refused for lacking a none effort, as MODEL-010 refused it

  Scenario Outline: A declared token budget is sent in the engine's own field
    Given a <engine> profile whose model declares a token budget from 128 to 32768
    When a turn is sent at a 4096-token budget
    Then the request carries <field> 4096
    And it is not refused as MODEL-010 refused budgets on local servers

    Examples:
      | engine       | field                         |
      | vLLM         | thinking_token_budget         |
      | TensorRT-LLM | thinking_token_budget         |
      | llama.cpp    | reasoning_budget_tokens       |
      | Strata       | reasoning_budget_tokens       |
      | SGLang       | custom_params.thinking_budget |

  Scenario: A profile-wide declaration applies to a model with none of its own
    Given a Strata profile that declares efforts "none, low, medium, xhigh" profile-wide
    And its model "qwen3.8-flash-next" has no declaration of its own
    When a turn is sent at effort "medium"
    Then the request carries reasoning_effort "medium"

  Scenario: A per-model declaration replaces the profile-wide one
    Given a llama.cpp profile that declares efforts "low, high" profile-wide
    And its model "gemma-4" declares efforts "none, low, medium, high, max"
    Then the levels offered for "gemma-4" are "none, low, medium, high, max"
    And nothing from the profile-wide declaration is merged in

  Scenario: An undeclared local model has no reasoning control
    Given a vLLM profile whose model "qwen3-32b" has no reasoning declaration, profile-wide or per model
    When a turn is sent at effort "high"
    Then the turn is refused before the request is sent
    And the error names "qwen3-32b" and says to declare its levels in the profile
    And neither reasoning_effort nor chat_template_kwargs is sent, where MODEL-010 sent both unchecked

  Scenario: LM Studio receives reasoning_effort on chat completions
    Given an LM Studio profile whose model declares efforts "low, medium, high"
    When a turn is sent at effort "medium"
    Then the /v1/chat/completions request carries reasoning_effort "medium"

  Scenario Outline: A declared part the engine has no field for is an error for that model only
    Given a <engine> profile whose model declares <part>
    And a vLLM profile whose model declares efforts "low, high"
    When the profiles are loaded
    Then the <engine> model's declaration is an error naming the profile, the model, <engine> and <part>
    And the vLLM model still offers "low, high"

    Examples:
      | engine    | part                                  |
      | TGI       | efforts "low, high"                   |
      | Ollama    | the template toggle "enable_thinking" |
      | LMDeploy  | a token budget                        |
      | LM Studio | a token budget                        |
      | Strata    | the template toggle "thinking"        |
      | vLLM      | decimal efforts from 0.0 to 0.99      |

  Scenario: A declaration on a profile with no engine is an error
    Given a local-server profile with no engine whose model declares efforts "low, high"
    When the profile is loaded
    Then the declaration is an error naming the profile and saying it names no engine
    And no engine is inferred from the base URL or the model id

  Scenario: A live report that differs from the declaration is reported and the declaration still applies
    Given a llama.cpp profile whose model declares efforts "none, low, high, max"
    And its GET /props reports reasoning_effort_levels "low, medium, high"
    When the model's capability is resolved
    Then a mismatch is reported naming the model, the declared "none, low, high, max" and the reported "low, medium, high"
    And the levels offered are "none, low, high, max"

  Scenario: A declared none is not a mismatch against llama.cpp
    Given a llama.cpp profile whose model declares efforts "none, low, high"
    And its GET /props reports reasoning_effort_levels "low, high"
    When the model's capability is resolved
    Then no mismatch is reported

  Scenario: Live data never stands in for a missing declaration
    Given a llama.cpp profile whose model has no reasoning declaration
    And its GET /props reports reasoning_effort_levels "low, high"
    When a turn is sent at effort "low"
    Then the turn is refused before the request is sent, saying to declare the model's levels in the profile

  Scenario: A live check that cannot run is reported and the declaration still applies
    Given an Ollama profile whose model declares efforts "none, low, medium, high"
    And POST /api/show cannot be reached
    When the model's capability is resolved
    Then a failed live check is reported naming the model and POST /api/show
    And the levels offered are "none, low, medium, high"

  Scenario: Ollama false matches a declared none
    Given an Ollama profile whose model declares efforts "none, low, medium, high"
    And its POST /api/show reports thinking values "false, low, medium, high"
    When the model's capability is resolved
    Then no mismatch is reported

  Scenario: LM Studio's off and on are compared with the declaration but change nothing sent
    Given an LM Studio profile whose model declares efforts "low, medium, high"
    And its GET /api/v1/models reports allowed_options "off, on"
    When the model's capability is resolved
    Then a mismatch is reported naming the model, the declared "low, medium, high" and the reported "off, on"
    When a turn is sent at effort "medium"
    Then the request carries reasoning_effort "medium"
