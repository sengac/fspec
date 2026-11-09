@done
@rust
@tui
@thinking-level
@model-selection
@high
@MODEL-012
Feature: Thinking level remembered per model
  """
  Persistence: rust/sessions/src/default_thinking_level_persistence.rs holds tui.defaultThinkingLevel as int 0..=3 (level_from_u8 / level_to_u8, MODEL-011 refusal via codelet_rpc_types::SAVED_DEFAULT_REFUSAL and ThinkingLevel::fits_saved_default in rpc-types/src/thinking.rs). It becomes per-model save/load over tui.thinkingLevels: load(model) -> Result<Option<ThinkingLevel>, String> (Err for an unreadable config, a non-object map or an unparseable entry; Ok(None) for no entry), save(model, level) read-modify-write of the RAW user file (BUG-193/BUG-461), ModelDefault removes the entry. load_config_with_dirs deep-merges project over user (common/src/fspec_config.rs deep_merge), so a string value per model is replaced whole. fits_saved_default and SAVED_DEFAULT_REFUSAL are removed
  Model key: ThinkingTarget::model() (providers/src/thinking_wire.rs for_provider_manager, from ProviderManager::selected_model_string: provider/model, or provider:profile/model for a profile) is the name thinking refusals already use and is cached on the session (BackgroundSession::thinking_target), so set_thinking_level_default (sessions/src/handle_impl/handle_impl_4.rs) and creation read the key from the same place the level is checked
  Apply on create/resume: sessions/src/session_creation_helper.rs (after set_thinking_target) is the one path for new, isolated and resumed sessions (session_manager/manager_impl_3.rs, _4.rs resume, _5.rs isolated). It loads the model's saved level, checks it with thinking_options::accepted against the cached target, and on refusal or a read error leaves ModelDefault and emits StreamChunk::UserNotification {severity: Warning} through BackgroundSession::handle_output (buffered, as provider_status.rs does), so the TUI shows why
  TUI: components/thinking_level_dialog/actions.rs set_default drops the fits_saved_default gate (numeric rows save the session's current value of that kind, else a refusal); app/dispatch_thinking_levels.rs seeds the (default) marker with the saved level for ThinkingOptions' model; app/dispatch_session_chrome.rs apply_default_thinking_level_if_needed and app/bootstrap.rs initialize_default_thinking_level (TUI-093 client-side apply of the global value) are removed, with refresh_session_chrome doing the plain get_thinking_level. RPC signatures are unchanged: set_thinking_level_default(session_id, level) now saves for the session's model
  Superseded behaviour in other features, updated alongside: default-thinking-level-persistence (TUI-002), default-thinking-level-shared-config-persistence (TUI-092), default-thinking-level-restore-parity (TUI-093), thinking-dialog-default-indicator (TUI-094), thinking-dialog-live-default-marker (TUI-095), the MODEL-011 scenario 'D saves only a level the saved default can hold', and their tests (sessions tui002/tui092/tui093/bug193/rpc425, fspec-tui tui093/tui095/model011/rpc027). Test files shared with COV-069 are coordinated through the supervisor before editing
  Model switch: sessions/src/handle_impl/handle_impl_6.rs set_model already caches the new model's ThinkingTarget; after the switch commits it runs the same apply as creation (saved level for the new model, checked; not offered or unreadable -> ModelDefault plus a Warning UserNotification; no saved level -> ModelDefault). The TUI repaints from get_thinking_level after ModelSelectionLoaded
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The saved thinking level is remembered per model (product owner, 2026-11-04, decision 6): D in the thinking dialog (set_thinking_level_default) saves the level for the session's current model only, keyed by the model as fspec names it in thinking errors (provider/model, profile-qualified for a profile model, e.g. anthropic/claude-opus-5-5 or openai:strata/DeepSeek-V4.1); saving for one model leaves every other model's saved level unchanged
  #   2. There is no global saved level: a model with no saved level of its own starts with no level set (model default, shown as Default), never with another model's level or a global value (decision 6 chose per-model memory over a single global default; no fallbacks, standing decision 2026-11-05)
  #   3. The saved level holds any level the model offers exactly as chosen: Off, On, a named effort, a numeric effort or a token budget, with nothing mapped; MODEL-011's refusal of D on a level other than off, low, medium or high is removed. D on a level the model does not offer is refused naming the model and its levels (an unadvertised level is refused, product owner 2026-11-05), and D on the Model default row removes the current model's saved level (BUG-455)
  #   4. D on a numeric row (token budget or numeric effort) saves the session's current value of that kind when the row holds it (the value the number field is pre-filled with, MODEL-011 rule 12); when the session holds no value of that kind, D is refused with a notice to enter a number first, and nothing is saved
  #   5. A saved level that cannot be read is an error, never loaded as Off or as any other level: an unreadable or invalid config, a tui.thinkingLevels that is not an object, or an entry for the model that is not a level leaves the session's level not set, and the session tells the user what could not be read and why (no silent defaults, standing decision 2026-11-05)
  #   6. When a session is created or resumed, its model's saved level is applied when the model offers it, checked exactly as set_thinking_level checks a level (MODEL-009 capability through MODEL-010's wire table). A saved level the model does not offer leaves the session's level not set (no thinking parameter is sent) and the session tells the user why, naming the model, the saved level and the levels the model takes or that it has no reasoning control (product owner, 2026-11-04, decision 7); the saved entry is kept, not deleted
  #   7. The daemon is the single place a saved level is applied, so every client (TUI, MCP, headless) gets the same level; the TUI no longer applies a saved level itself at startup or when a session becomes active (TUI-093's client-side apply read the single global value), and it repaints the header from get_thinking_level as today
  #   8. The thinking dialog marks only the current model's saved level with the (default) marker: no marker when the current model has no saved level or its saved level is not offered, and a level saved for another model never marks a row; D moves the marker live (TUI-095)
  #   9. Storage: the user-scope fspec-config.json key tui.thinkingLevels, an object from the model key to the level as one string: off, on, effort:<name>, numeric:<number> or budget:<tokens> (a single string so the project-over-user deep merge replaces a model's level whole and never mixes two levels); it is read from the merged config like other tui keys and written to the user file preserving sibling keys (BUG-193, BUG-461). tui.defaultThinkingLevel (int 0..=3) is no longer read or written and is never mapped onto a level (standing decision 2026-11-05: no legacy mappings, only the new system; a single global value also cannot become per-model levels without inventing which models it belongs to)
  #   10. Product owner, 2026-11-05: option (a). On a model switch the session takes the new model's saved level: applied when the new model offers it; when it does not, the session's level is not set and the user is told why; when the new model has no saved level, the level becomes not set (model default, header shows Default).
  #   11. Product owner, 2026-11-05: option (a). tui.defaultThinkingLevel is ignored and left in the file: no notice is shown and it is not removed.
  #   12. Product owner, 2026-11-05, confirming three derived decisions: the saved level is per model only, with no global default; the old int tui.defaultThinkingLevel is not migrated; only D (set_thinking_level_default) saves a level, and choosing a level in the dialog or with /thinking does not save it
  #
  # EXAMPLES:
  #   1. On anthropic/claude-opus-5-5 the user highlights xhigh and presses D: tui.thinkingLevels gets anthropic/claude-opus-5-5 = xhigh, the (default) marker moves to xhigh, and the next new session on anthropic/claude-opus-5-5 starts at xhigh with the header showing xhigh
  #   2. Saved levels are anthropic/claude-opus-5-5 = xhigh and openai/gpt-5.5 = none: a new session on openai/gpt-5.5 starts at none, and a new session on google/gemini-2.5-pro, which has no saved level, starts with no level set and the header shows Default
  #   3. On anthropic/claude-haiku-4-5 the session's level is a token budget of 8000: D on the Token budget row saves anthropic/claude-haiku-4-5 = budget 8000; on google/gemini-2.5-pro with no budget chosen, D on the Token budget row is refused with a notice to enter a number first and nothing is saved
  #   4. On openai/gpt-5.5 with a saved level none, D on the Model default row removes the openai/gpt-5.5 entry; the anthropic/claude-opus-5-5 entry stays
  #   5. The saved level for openai/gpt-5 is xhigh, which openai/gpt-5 does not offer: a new session on openai/gpt-5 starts with no level set, the header shows Default, the session warns that the saved thinking level xhigh for openai/gpt-5 is not offered and lists minimal, low, medium, high, and the entry stays in the config
  #   6. tui.thinkingLevels has openai/gpt-5 = the bare number 3: a new session on openai/gpt-5 starts with no level set and warns that its saved thinking level in tui.thinkingLevels cannot be read; it is never loaded as Off or high
  #   7. The user config has tui.defaultThinkingLevel 3 and no tui.thinkingLevels: a new session on anthropic/claude-opus-5-5 starts with no level set; nothing maps 3 to high
  #   8. A Strata profile model openai:strata/DeepSeek-V4.1 has the saved numeric effort 40: a new session on it starts at 40 and its turn sends reasoning_effort 40
  #   9. The dialog on openai/gpt-5.5 with the saved level none marks none (default); the dialog on openai/gpt-5 whose saved xhigh is not offered marks no row; the dialog on google/gemini-2.5-pro marks no row although other models have saved levels
  #   10. The TUI starts with a resumed session on anthropic/claude-opus-5-5 that the daemon applied xhigh to: the TUI sets no level itself and the header shows xhigh from get_thinking_level; a level the user then picks with /thinking is not replaced when the session regains focus
  #   11. The session on anthropic/claude-opus-5-5 is at xhigh and the saved levels are openai/gpt-5.5 = none and openai/gpt-5 = xhigh: switching to openai/gpt-5.5 sets none; switching to openai/gpt-5 leaves the level not set (Default) and warns that the saved xhigh is not offered, listing minimal, low, medium, high; switching to google/gemini-2.5-pro, which has no saved level, leaves the level not set and the header shows Default
  #
  # QUESTIONS (ANSWERED):
  #   Q: When a session switches to another model (model picker or set_model), what happens to its thinking level? (a) The session takes the new model's saved level (applied if the model offers it; if not, the level is not set and the user is told why, as decision 7), and when the new model has no saved level the level becomes not set (model default, header shows Default). (b) As (a), but when the new model has no saved level the session keeps its current level, and the next turn is refused if the new model does not offer it (MODEL-011's behaviour today). (c) A model switch never changes the level; saved levels apply only when a session is created or resumed. Recommendation: (a), so each model runs at its own saved level or its model default and no level is carried onto a model that may not take it.
  #   A: Product owner, 2026-11-05: option (a). On a model switch the session takes the new model's saved level: applied when the new model offers it; when it does not, the session's level is not set and the user is told why; when the new model has no saved level, the level becomes not set (model default, header shows Default).
  #
  #   Q: tui.defaultThinkingLevel (int 0..=3) is not migrated (standing decision: no legacy mappings) and is no longer read. What should fspec do when a config still has it? (a) Ignore it and leave it in the file. (b) Leave it, and when a session starts tell the user that tui.defaultThinkingLevel is no longer read and that D now saves a level per model. (c) Remove it from the user config the next time D saves a level. (b) and (c) can be combined. Recommendation: (b) together with (c): the notice shows until the next D removes the old key, so the user's old setting is never dropped silently.
  #   A: Product owner, 2026-11-05: option (a). tui.defaultThinkingLevel is ignored and left in the file: no notice is shown and it is not removed.
  #
  # ========================================
  Background: User Story
    As a fspec user who switches between models
    I want to have the thinking level I save remembered for each model, and be told when a saved level no longer fits its model
    So that each model starts at the level I chose for it, in that model's own levels, without fspec mapping or silently dropping a level

  Scenario: D saves the level for the current model only
    Given the saved thinking level for openai/gpt-5.5 is none
    And the session's model is anthropic/claude-opus-5-5
    When the user saves xhigh with D
    Then tui.thinkingLevels holds effort:xhigh for anthropic/claude-opus-5-5 and still holds effort:none for openai/gpt-5.5
    And a new session on anthropic/claude-opus-5-5 starts at xhigh

  Scenario: A model with no saved level starts with no level set
    Given the saved thinking levels are xhigh for anthropic/claude-opus-5-5 and none for openai/gpt-5.5
    When a new session starts on openai/gpt-5.5
    Then its thinking level is none
    When a new session starts on google/gemini-2.5-pro
    Then its thinking level is not set

  Scenario: D on a numeric row saves the value the session holds
    Given the session's model is anthropic/claude-haiku-4-5 and its thinking level is a token budget of 8000
    When the user presses D on the token budget row
    Then the dialog asks to save a token budget of 8000
    And tui.thinkingLevels holds budget:8000 for anthropic/claude-haiku-4-5
    Given the session's model is google/gemini-2.5-pro and its thinking level is not set
    When the user presses D on the token budget row
    Then D is refused with a notice to choose a token budget first and nothing is saved

  Scenario: D on Model default removes only the current model's saved level
    Given the saved thinking levels are none for openai/gpt-5.5 and xhigh for anthropic/claude-opus-5-5
    And the session's model is openai/gpt-5.5
    When the user saves Model default with D
    Then tui.thinkingLevels has no entry for openai/gpt-5.5
    And tui.thinkingLevels still holds effort:xhigh for anthropic/claude-opus-5-5

  Scenario: Saving a level the model does not offer is refused
    Given the session's model is openai/gpt-5
    When set_thinking_level_default is called with xhigh
    Then it is refused naming openai/gpt-5 and listing minimal, low, medium, high
    And tui.thinkingLevels has no entry for openai/gpt-5

  Scenario: A saved level the model does not offer leaves the level not set and tells the user why
    Given the saved thinking level for openai/gpt-5 is xhigh
    When a new session starts on openai/gpt-5
    Then its thinking level is not set
    And the session warns that the saved thinking level xhigh for openai/gpt-5 is not offered, listing minimal, low, medium, high
    And tui.thinkingLevels still holds effort:xhigh for openai/gpt-5

  Scenario: A saved level that cannot be read is reported and never loaded as Off
    Given tui.thinkingLevels holds the bare number 3 for openai/gpt-5
    When a new session starts on openai/gpt-5
    Then its thinking level is not set
    And the session warns that the saved thinking level for openai/gpt-5 in tui.thinkingLevels cannot be read

  Scenario: The old tui.defaultThinkingLevel is ignored and left in the file
    Given the user config has tui.defaultThinkingLevel 3 and no tui.thinkingLevels
    When a new session starts on anthropic/claude-opus-5-5
    Then its thinking level is not set and the session shows no notice
    When the user saves xhigh with D on anthropic/claude-opus-5-5
    Then the user config still has tui.defaultThinkingLevel 3

  Scenario: A profile model's numeric effort is remembered under its profile-qualified name
    Given the saved thinking level for openai:strata/DeepSeek-V4.1 is the numeric effort 40
    And the Strata profile model DeepSeek-V4.1 declares whole-number efforts 1 to 100
    When its saved level is applied
    Then its thinking level is the numeric effort 40
    And its turn sends reasoning_effort 40

  Scenario: Switching model takes the new model's saved level
    Given the session on anthropic/claude-opus-5-5 is at xhigh
    And the saved thinking levels are none for openai/gpt-5.5 and xhigh for openai/gpt-5
    When the user switches the session to openai/gpt-5.5
    Then its thinking level is none
    When the user switches the session to openai/gpt-5
    Then its thinking level is not set and the session warns that the saved thinking level xhigh for openai/gpt-5 is not offered
    When the user switches the session to google/gemini-2.5-pro
    Then its thinking level is not set

  Scenario: The dialog marks only the current model's saved level
    Given the saved thinking levels are none for openai/gpt-5.5 and xhigh for openai/gpt-5
    When the user opens the thinking dialog on openai/gpt-5.5
    Then none carries the (default) marker
    When the user opens the thinking dialog on openai/gpt-5
    Then no row carries the (default) marker
    When the user opens the thinking dialog on google/gemini-2.5-pro
    Then no row carries the (default) marker

  Scenario: The TUI applies no saved level itself
    Given the saved thinking level for anthropic/claude-opus-5-5 is xhigh
    And the daemon's session on anthropic/claude-opus-5-5 is at high
    When the TUI starts with that session active and the session regains focus
    Then the TUI sends no set_thinking_level
    And the header shows high from get_thinking_level

  Scenario: Choosing a level does not save it
    Given the session's model is anthropic/claude-opus-5-5 with no saved level
    When the user chooses high with /thinking
    Then the session's thinking level is high
    And tui.thinkingLevels has no entry for anthropic/claude-opus-5-5
