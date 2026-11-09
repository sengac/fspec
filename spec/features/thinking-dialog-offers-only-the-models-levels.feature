@BUG-582
@done
@rust
@tui
@thinking-level
@model-selection
@high
@MODEL-011
Feature: Thinking dialog and /thinking offer only the current model's levels
  """
  Architecture notes:
  - The thinking wire table (ThinkingWire, ThinkingTarget, TurnLevel and the forms) moves from codelet-agent-loop thinking_config into codelet-providers (thinking_wire), so the sessions crate that serves set_thinking_level can reach it; agent-loop's thinking_config re-exports it. ThinkingTarget::offer lists the levels the model offers by probing its own config_for: Off where it has an off form that is not its advertised none effort, each advertised effort, the numeric effort range, the token budget, and On; a target with no capability (Ok(None), whatever its source) or an unreadable one has no reasoning control. ThinkingTarget::accepts refuses a level the target does not offer before anything is stored.
  - codelet_rpc_types::ThinkingLevel becomes open: Off, ModelDefault, On, Effort(name), Numeric(JSON number), Budget(tokens); Low/Medium/High are gone (no legacy mapping). New ThinkingOptions (NoReasoningControl {model, reason} | Levels {model, choices, takes_off}) and ThinkingChoice (Off, Effort, NumericEffort {kind, min, max}, TokenBudget {min, max}, On) carry the offer to clients; ThinkingOptions::resolve_command turns a /thinking argument into a level or the refusal.
  - BackgroundSession holds the chosen level as a ThinkingLevel (no u8, no clamp) and caches the selected model's ThinkingTarget next to its model limits (set at creation, on set_model and on each turn's runtime-settings refresh), so set_thinking_level and the new get_thinking_options query work while a turn holds the session's inner lock. set_thinking_level refuses a level the cached target does not accept, with the target's reason. The turn converts the stored level to a TurnLevel and sends it through the wire table unchanged.
  - The new get_thinking_options RPC runs through SessionManagerHandle, the FspecService declaration and implementation, the MCP dispatch catalog and both TUI transports. The TUI opens the thinking dialog from the options it fetches; /thinking resolves its argument against them; D keeps tui.defaultThinkingLevel's int 0..=3 (off, low, medium, high) until MODEL-012 and refuses any other level. (MODEL-012 has since replaced it: D saves any offered level for the current model under tui.thinkingLevels, and the scenario for the old refusal is gone.)
  - MODEL-017 plug-in point: ThinkingTarget::for_provider_manager gives ProviderType::Custom (Rhai) models capability Ok(None); MODEL-017 resolves their declared thinkingLevels there, like local_profile does for profiles, and the dialog and /thinking follow with no change. Until then a target with no capability has no reasoning control in the dialog and /thinking, while the turn keeps MODEL-010's unchecked sending for a level a session already holds (product owner, 2026-11-05). MODEL-017 now resolves custom provider models' declared thinkingLevels there and ends that unchecked sending for them; a custom provider model that declares nothing has its level refused at the turn.
  - Research: spec/attachments/MODEL-009/thinking-levels-research.md section 7.3 and the product owner decisions in section 8 (2026-11-04) and 2026-11-05.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The thinking dialog lists every level the session's current model offers, in canonical order, with nothing behind a More… row (product owner, 2026-11-04, decision 10): Off first where the model has an off form, then its named efforts in MODEL-009's canonical order (none < minimal < low < medium < high < xhigh < max < ultra, other advertised names after them in the model's order), then its numeric effort range, then its token budget, then On for a toggle-only model; BUG-455's Model default stays the last row
  #   2. A level is offered only when the model advertises it and the dispatched provider has a form for it in MODEL-010's wire table, so the dialog never lists a level the next turn would refuse (derived from MODEL-010 strict refusal): for example Off is not offered on claude-opus-5-5, whose thinking cannot be turned off, and On is not offered on Gemini or Z.AI, which have no On form
  #   3. Off is listed only where the model's off form is not one of its advertised efforts: on a model that advertises the none effort, Off and none send the same thing (MODEL-010 rule 10, MODEL-016 rule 18), so the dialog lists none and no separate Off row; /thinking off is still accepted wherever the model has an off form
  #   4. A model with a token budget takes a numeric budget within its advertised min..max (an advertised min with no max has no upper bound); fspec invents no named levels or budget table for it, and a budget outside the range is refused naming the model and the range (product owner, 2026-11-04, decision 9; standing decision 2026-11-05)
  #   5. MODEL-016's numeric efforts (whole numbers or decimals within the declared min..max) and the toggle-only On and Off are selectable in the dialog and with /thinking, and are sent unchanged (product owner, 2026-11-05)
  #   6. /thinking <value> accepts any level the current model offers, matched word for word against the advertised names after lowercasing; a value the model does not offer (including the old alias med) is refused with a notice naming the model and listing the levels it takes, and the session's level is unchanged (an unadvertised level is refused, product owner, 2026-11-05). /thinking default still clears the level (BUG-455) and bare /thinking opens the dialog
  #   7. A model with no reasoning control (a catalog model with no reasoning options, or a local profile model that declares no levels) says so explicitly: the dialog shows that the model has no reasoning control and offers no level rows, and /thinking <level> is refused with the same reason (product owner, 2026-11-05)
  #   8. The session stores the chosen level exactly as chosen: not set (model default), Off, On, a named effort, a numeric effort or a token budget. It is no longer a u8 clamped to 0..=3 or one of Off/Low/Medium/High, and the turn sends it unchanged through MODEL-010's wire table; there is no legacy mapping (product owner, 2026-11-04/05)
  #   9. The RPC thinking-level type carries the open level (not set, Off, On, a named effort, a numeric effort, a token budget) for set_thinking_level, get_thinking_level and the session state snapshot, and a new query returns the levels the session's current model offers, or that it has no reasoning control, so the dialog and /thinking use the backend's single source of truth
  #   10. The session header shows the chosen level as chosen (the effort name, the numeric effort, the budget in tokens, On), Default when no level is set, and nothing for Off, as today
  #   11. Scope (MODEL-012 owns per-model memory): this card does not change what happens to a session's level when its model changes; the next turn is checked and refused naming the model and its levels as MODEL-010 does. The dialog opened on a level the current model does not offer marks no row as current and starts the selection on the first row
  #   12. Scope (MODEL-012 owns the saved default): tui.defaultThinkingLevel stays the int 0..=3 meaning off, low, medium, high until MODEL-012 replaces it; D (set default) on any other level is refused with a notice saying the saved default holds only off, low, medium or high until it is remembered per model, and nothing is mapped onto the old values
  #   13. Product owner, 2026-11-05: the recommendation. One row per numeric kind showing its range (e.g. 'Token budget 128–32768', 'Effort 1–100'); Enter opens an in-dialog number field, pre-filled with the session's current value of that kind if there is one; Enter commits a number within the range, otherwise the field shows the refusal and stays open; Esc goes back to the list. No Left/Right stepping.
  #   14. Product owner, 2026-11-05: the recommendation. '/thinking 8000' is the model's token budget or its numeric effort when the model takes only one of the two; it is refused as ambiguous, naming both ranges, when the model takes both. '/thinking budget 8000' and '/thinking effort 40' are always accepted on any model that takes that kind.
  #   15. Product owner, 2026-11-05: option (c). Rhai provider models will declare thinkingLevels like MODEL-016 profile models in a new card, MODEL-017 (not in MODEL-011's scope). Until a model declares, it has no reasoning control: the dialog says so and offers only Model default, and /thinking refuses levels. The same applies to cloud models missing from models.dev. MODEL-011 implements 'no capability means no reasoning control' in the dialog and /thinking for these models.
  #   16. Scope of the 2026-11-05 answer on unknown levels: MODEL-011 changes only what the dialog and /thinking offer for Rhai custom-provider models and cloud models missing from models.dev (no capability, so no reasoning control); the turn's wire table keeps MODEL-010's unchecked sending for a level such a session already holds (a saved default or a level kept across a model switch, both MODEL-012's), until MODEL-017 gives those models declarations
  #
  # EXAMPLES:
  #   1. openai/gpt-5.5 advertises [none, low, medium, high, xhigh]: the dialog lists none, low, medium, high, xhigh, Model default, with no separate Off row and no More… row
  #   2. openai/gpt-5 advertises [minimal, low, medium, high] and no none: the dialog lists minimal, low, medium, high, Model default; Off is not offered and /thinking off is refused naming openai/gpt-5 and listing minimal, low, medium, high
  #   3. anthropic/claude-opus-5-5 advertises [low, medium, high, xhigh, max] and cannot turn thinking off: the dialog lists low, medium, high, xhigh, max, Model default, with no Off row; choosing xhigh sends output_config effort xhigh
  #   4. anthropic/claude-haiku-4-5 advertises only a token budget with min 1024: the dialog offers Off and a token budget of at least 1024; a budget of 8000 is stored and sent as thinking {type: enabled, budget_tokens: 8000}, and 512 is refused naming the model and the minimum 1024
  #   5. google/gemini-2.5-pro advertises a token budget of 128 to 32768: /thinking with 40000 is refused naming the model and the range 128 to 32768, and the session keeps its level
  #   6. A Strata profile model DeepSeek-V4.1 declares efforts [none, low, medium, high, max] and whole-number efforts 1 to 100: the dialog lists none, low, medium, high, max, the numeric effort 1 to 100, Model default; choosing 40 sends reasoning_effort 40
  #   7. A TensorRT-LLM profile model declaring only toggle enable_thinking: the dialog lists Off, On, Model default; On sends chat_template_kwargs {enable_thinking: true}
  #   8. zai/glm-4.6 (toggle only): the dialog lists Off and Model default; On is not offered because Z.AI has no On form
  #   9. /thinking xhigh on openai/gpt-5.5 sets the session's level to xhigh and the header shows xhigh; /thinking med on the same model is refused listing none, low, medium, high, xhigh
  #   10. A vLLM profile model qwen3-32b with no declaration: the dialog says qwen3-32b has no reasoning control and offers no level rows; /thinking high is refused with the same reason
  #   11. The session's level is xhigh and the user switches to openai/gpt-5 (no xhigh): the level stays xhigh, the next turn is refused naming openai/gpt-5 and its levels, and the dialog opens with no row marked current and the selection on minimal
  #   12. With xhigh highlighted, pressing D is refused with a notice that the saved default holds only off, low, medium or high until it is remembered per model; with high highlighted, D saves high as the default as today
  #   13. A Rhai custom-provider model (config declares only supports_thinking) and a cloud model missing from models.dev: the dialog says the model has no reasoning control and offers only Model default; /thinking high is refused with the same reason
  #   14. On anthropic/claude-haiku-4-5 (budget min 1024) '/thinking 8000' sets a token budget of 8000; on a Strata model declaring whole-number efforts 1 to 100 and a token budget 128 to 32768, '/thinking 50' is refused as ambiguous naming both ranges, while '/thinking effort 50' and '/thinking budget 4096' are accepted
  #   15. On google/gemini-2.5-pro the dialog row 'Token budget 128–32768' opens a number field; typing 40000 and Enter shows the refusal naming the range and keeps the field open, typing 8000 and Enter sets the budget and closes the dialog, and Esc returns to the list
  #
  # QUESTIONS (ANSWERED):
  #   Q: How does the user enter a numeric level in the thinking dialog? A token budget (claude-haiku-4-5: min 1024, no max; gemini-2.5-pro: 128 to 32768) and a numeric effort (Strata whole numbers 1 to 100, SGLang decimals 0.0 to 0.99) each need a number, and §8 decides the range but not the input. Recommendation: one row per numeric kind showing its range (e.g. 'Token budget 128–32768', 'Effort 1–100'); Enter on it opens a number field inside the dialog, pre-filled with the session's current value of that kind if any; Enter commits a number within the range, otherwise the field shows the refusal and stays open; Esc goes back to the list. Alternative: Left/Right step the value on the row, which would also need a step size per range.
  #   A: Product owner, 2026-11-05: the recommendation. One row per numeric kind showing its range (e.g. 'Token budget 128–32768', 'Effort 1–100'); Enter opens an in-dialog number field, pre-filled with the session's current value of that kind if there is one; Enter commits a number within the range, otherwise the field shows the refusal and stays open; Esc goes back to the list. No Left/Right stepping.
  #
  #   Q: How does /thinking take a number? Recommendation: '/thinking 8000' is the model's token budget when it takes a budget and no numeric effort, its numeric effort when it takes a numeric effort and no budget, and is refused as ambiguous (naming both ranges) when it takes both (a Strata declaration may have whole-number efforts 1 to 100 and a token budget); then '/thinking budget 8000' or '/thinking effort 40' says which, and those two forms are accepted on any model that takes that kind. Alternative: a budget always needs the 'budget' word.
  #   A: Product owner, 2026-11-05: the recommendation. '/thinking 8000' is the model's token budget or its numeric effort when the model takes only one of the two; it is refused as ambiguous, naming both ranges, when the model takes both. '/thinking budget 8000' and '/thinking effort 40' are always accepted on any model that takes that kind.
  #
  #   Q: What do the dialog and /thinking offer for a model whose levels fspec cannot know: Rhai custom-provider models (their config declares only supports_thinking) and cloud models missing from the models.dev catalog? MODEL-010 sends their level unchecked, MODEL-016 kept that for Rhai, and the bundled codex.rhai template builds its reasoning object from the level. Options: (a) treat them as having no reasoning control (the dialog says so and offers only Model default), which stops codex.rhai users from choosing a level; (b) offer Off and the canonical named efforts none to ultra, labelled as unchecked, and send them unchecked as today; (c) let a Rhai provider model declare thinkingLevels like a MODEL-016 profile model (new scope, possibly its own card) and use (a) until then.
  #   A: Product owner, 2026-11-05: option (c). Rhai provider models will declare thinkingLevels like MODEL-016 profile models in a new card, MODEL-017 (not in MODEL-011's scope). Until a model declares, it has no reasoning control: the dialog says so and offers only Model default, and /thinking refuses levels. The same applies to cloud models missing from models.dev. MODEL-011 implements 'no capability means no reasoning control' in the dialog and /thinking for these models.
  #
  # ========================================
  Background: User Story
    As a developer choosing how much the model thinks
    I want to pick from the thinking dialog or /thinking only the levels the current model takes, including named efforts, numeric efforts, token budgets and On/Off
    So that every level I can choose is one the model accepts, and the session sends exactly that level

  Scenario: An OpenAI model with a none effort lists its efforts in canonical order and no separate Off
    Given the session's model is openai/gpt-5.5, which advertises the efforts none, low, medium, high and xhigh
    When the user opens the thinking dialog
    Then the dialog lists none, low, medium, high, xhigh and Model default, in that order
    And the dialog has no Off row and no More… row

  Scenario: A model without an off form offers no Off and refuses /thinking off
    Given the session's model is openai/gpt-5, which advertises the efforts minimal, low, medium and high and no none
    When the user opens the thinking dialog
    Then the dialog lists minimal, low, medium, high and Model default
    And /thinking off is refused naming openai/gpt-5 and listing minimal, low, medium, high

  Scenario: A Claude model whose thinking cannot be turned off offers its efforts without Off
    Given the session's model is anthropic/claude-opus-5-5, which advertises the efforts low, medium, high, xhigh and max
    When the user opens the thinking dialog
    Then the dialog lists low, medium, high, xhigh, max and Model default
    And choosing xhigh sends output_config effort xhigh

  Scenario: A budget-only model takes a token budget at or above its minimum
    Given the session's model is anthropic/claude-haiku-4-5, which advertises only a token budget with a minimum of 1024
    When the user opens the thinking dialog
    Then the dialog lists Off, Token budget 1024 or more and Model default
    And a budget of 8000 is stored and sent as thinking type enabled with budget_tokens 8000
    And a budget of 512 is refused naming anthropic/claude-haiku-4-5 and the minimum 1024, and the session keeps its level

  Scenario: The token budget row takes a number in the dialog
    Given the session's model is google/gemini-2.5-pro, which advertises a token budget of 128 to 32768
    And the thinking dialog is open on the row Token budget 128–32768
    When the user presses Enter, types 40000 and presses Enter
    Then the number field shows the refusal naming the range 128 to 32768 and stays open
    When the user clears the field, types 8000 and presses Enter
    Then the session's level is a token budget of 8000 and the dialog closes
    And pressing Esc in the number field returns to the list without changing the level

  Scenario: A local model's numeric efforts are offered and sent unchanged
    Given the session's model is a Strata profile model DeepSeek-V4.1 declaring the efforts none, low, medium, high and max and whole-number efforts 1 to 100
    When the user opens the thinking dialog
    Then the dialog lists none, low, medium, high, max, Effort 1–100 and Model default
    And choosing the numeric effort 40 sends reasoning_effort 40

  Scenario: A toggle-only local model offers Off and On
    Given the session's model is a TensorRT-LLM profile model declaring only the template toggle enable_thinking
    When the user opens the thinking dialog
    Then the dialog lists Off, On and Model default
    And choosing On sends chat_template_kwargs enable_thinking true

  Scenario: A Z.AI toggle-only model offers only Off
    Given the session's model is zai/glm-4.6, which advertises only a thinking toggle
    When the user opens the thinking dialog
    Then the dialog lists Off and Model default
    And /thinking on is refused naming zai/glm-4.6

  Scenario: /thinking sets an advertised effort and refuses an unadvertised one
    Given the session's model is openai/gpt-5.5
    When the user submits /thinking xhigh
    Then the session's level is xhigh and the header shows xhigh
    When the user submits /thinking med
    Then the notice says openai/gpt-5.5 does not take med and lists none, low, medium, high, xhigh
    And the session's level is still xhigh

  Scenario: /thinking with a number takes the one numeric kind the model offers
    Given the session's model is anthropic/claude-haiku-4-5, which takes only a token budget
    When the user submits /thinking 8000
    Then the session's level is a token budget of 8000
    Given the session's model is a Strata model declaring whole-number efforts 1 to 100 and a token budget of 128 to 32768
    When the user submits /thinking 50
    Then the notice says 50 is ambiguous and names the effort range 1 to 100 and the token budget 128 to 32768
    When the user submits /thinking effort 50
    Then the session's level is the numeric effort 50
    When the user submits /thinking budget 4096
    Then the session's level is a token budget of 4096

  Scenario: A local profile model without a declaration has no reasoning control
    Given the session's model is a vLLM profile model qwen3-32b that declares no thinking levels
    When the user opens the thinking dialog
    Then the dialog says qwen3-32b has no reasoning control and lists only Model default
    And /thinking high is refused with the same reason

  Scenario: Rhai provider models and models missing from the catalog have no reasoning control
    Given the session's model is a Rhai custom-provider model or a cloud model missing from models.dev
    When the user opens the thinking dialog
    Then the dialog says the model has no reasoning control and lists only Model default
    And /thinking high is refused with the same reason

  Scenario: The dialog opened on a level the model does not offer marks no row current
    Given the session's level is xhigh and its model is now openai/gpt-5, which does not offer xhigh
    When the user opens the thinking dialog
    Then no row is marked current and the selection starts on minimal
    And a turn at xhigh is refused naming openai/gpt-5 and listing minimal, low, medium, high

  Scenario: The session stores the chosen level exactly and the turn sends it unchanged
    Given a session whose model is anthropic/claude-haiku-4-5
    When the user chooses a token budget of 8000
    Then get_thinking_level returns a token budget of 8000
    And the turn sends thinking type enabled with budget_tokens 8000
    When the user chooses a budget of 512
    Then the choice is refused naming the minimum 1024 and get_thinking_level still returns a token budget of 8000
    When the user chooses Model default
    Then get_thinking_level returns Model default and the turn sends no thinking parameter
