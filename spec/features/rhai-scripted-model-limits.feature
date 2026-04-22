@done
@provider-abstraction
@configuration
@providers
@rust
@config
@rig
@PROV-095
Feature: Allow Rhai custom provider scripts to set model context window and max output tokens

  """
  Uses the new optional Rhai function `get_model_limits(config) -> Map` with shape `#{ context_window?: i64, max_output_tokens?: i64, compaction_threshold?: #{ type: "tokens"|"percentage", value: i64 } }`.
  Implemented in codelet/providers/src/custom/provider.rs (RhaiCustomProvider::new invokes the script) and codelet/providers/src/custom/rhai_call.rs or similar. Missing function is detected via rhai::Engine::call_fn returning FunctionNotFound — treated as 'no-op, use JSON defaults'.
  New accessor `RhaiCustomProvider::script_compaction_threshold() -> Option<(String, u64)>` exposes the parsed threshold so the NAPI session-creation path (codelet/napi/src/session_manager.rs) can call `ProviderManager::set_compaction_threshold_override` at the same moment it calls `override_model_limits`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rhai script MAY define an optional function `get_model_limits(config) -> Map` that returns a map with at least one of `context_window` (integer) and `max_output_tokens` (integer)
  #   2. `config` passed to `get_model_limits` is the same config map (`name`, `base_url`, `model`, `model_alias`) already passed to `build_url` / `build_headers`, so the script can branch per model alias
  #   3. When `get_model_limits` returns a valid map, the returned `context_window` / `max_output_tokens` override the JSON ModelDef values for the selected alias
  #   4. When `get_model_limits` is not defined in the script, `RhaiCustomProvider` falls back to the existing JSON ModelDef values (backward compatible — no regression for existing scripts)
  #   5. When the returned map only contains one of the two keys, only that field is overridden; the other falls back to the JSON ModelDef value
  #   6. Non-positive or non-integer values returned by `get_model_limits` are rejected and the JSON ModelDef value is used (with a log warning naming the provider + offending key)
  #   7. The resolved (script-overridden or JSON) value is what `RhaiCustomProvider::context_window()` and `RhaiCustomProvider::max_output_tokens()` return, so the existing propagation path to `ProviderManager` / NAPI / the TUI SessionHeader badge needs no new wiring
  #   8. Once, at provider construction. `get_model_limits(config)` is invoked synchronously from `RhaiCustomProvider::new` and the resolved values are cached in the struct fields for the provider's lifetime. Mirrors how model selection (model_id) is stable per provider instance today.
  #   9. Yes — `get_model_limits` MAY also return an optional `compaction_threshold` entry. Shape: `compaction_threshold: #{ type: "tokens", value: N }` (absolute tokens) or `compaction_threshold: #{ type: "percentage", value: P }` (percentage of context_window, P in 1..=100). When present, RhaiCustomProvider surfaces this through a new accessor that the NAPI layer calls to populate ProviderManager::set_compaction_threshold_override(Some((type, value))).
  #
  # EXAMPLES:
  #   1. A claude-rhai.rhai script defines `fn get_model_limits(config) { #{ context_window: 400000, max_output_tokens: 128000 } }`; the JSON ModelDef for `opus-4.7` has `context_window: 128000`; the constructed RhaiCustomProvider reports `context_window() == 400000` and `max_output_tokens() == 128000`
  #   2. A legacy claude-rhai.rhai script does NOT define `get_model_limits`; the JSON ModelDef says `context_window: 200000`; the constructed RhaiCustomProvider still reports `context_window() == 200000` (backward-compatible fallback)
  #   3. A script returns `#{ context_window: 400000 }` (no max_output_tokens key); the JSON ModelDef has `context_window: 128000` and `max_output_tokens: 4096`; the provider reports `context_window() == 400000` and `max_output_tokens() == 4096`
  #   4. A script returns `#{ context_window: -1 }`; the provider logs a warning naming `claude-rhai.context_window` and falls back to the JSON ModelDef value
  #   5. A script branches on `config.model_alias` and returns `400000` for `opus-4.7` and `200000` for `opus-4.6`; each constructed provider reports its own alias-specific context window
  #   6. A script returns `#{ context_window: 400000, compaction_threshold: #{ type: "tokens", value: 200000 } }`; at session creation the NAPI layer reads these from the RhaiCustomProvider and wires them into the ProviderManager so the TUI badge shows 400k and compaction triggers at 200k
  #   7. A script returns `#{ compaction_threshold: #{ type: "percentage", value: 75 } }`; context_window comes from the JSON ModelDef (1_000_000); compaction triggers at 750k (75% of 1M)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should `get_model_limits` be synchronous (called once at RhaiCustomProvider::new) or should it be re-invoked per request to support dynamic caps? Recommendation: synchronous + cached at construction — mirrors how build_headers/build_url are invoked (fresh Scope per call but model selection is stable for the lifetime of a RhaiCustomProvider).
  #   A: Once, at provider construction. `get_model_limits(config)` is invoked synchronously from `RhaiCustomProvider::new` and the resolved values are cached in the struct fields for the provider's lifetime. Mirrors how model selection (model_id) is stable per provider instance today.
  #
  #   Q: Should the Rhai script be able to override the compaction threshold too, or only the context_window / max_output_tokens (letting the existing CTX-007/-008 per-model threshold resolver take it from there)?
  #   A: Yes — `get_model_limits` MAY also return an optional `compaction_threshold` entry. Shape: `compaction_threshold: #{ type: "tokens", value: N }` (absolute tokens) or `compaction_threshold: #{ type: "percentage", value: P }` (percentage of context_window, P in 1..=100). When present, RhaiCustomProvider surfaces this through a new accessor that the NAPI layer calls to populate ProviderManager::set_compaction_threshold_override(Some((type, value))).
  #
  # ========================================

  Background: User Story
    As a fspec user running a Rhai custom provider
    I want to set the per-model context_window and max_output_tokens from inside my Rhai script
    So that the TUI badge and compaction threshold reflect the real limits my script expects, without having to edit the sibling JSON file

  Scenario: Script sets both context_window and max_output_tokens
    Given a custom Rhai provider named "claude-rhai" with JSON ModelDef "opus-4.7" declaring context_window 128000 and max_output_tokens 4096
    And the Rhai script defines "fn get_model_limits(config) { #{ context_window: 400000, max_output_tokens: 128000 } }"
    When a RhaiCustomProvider is constructed for model alias "opus-4.7"
    Then RhaiCustomProvider.context_window() returns 400000
    And RhaiCustomProvider.max_output_tokens() returns 128000

  Scenario: Legacy script without get_model_limits falls back to JSON ModelDef values
    Given a custom Rhai provider named "claude-rhai" with JSON ModelDef "opus-4.6" declaring context_window 200000 and max_output_tokens 8192
    And the Rhai script does NOT define a "get_model_limits" function
    When a RhaiCustomProvider is constructed for model alias "opus-4.6"
    Then RhaiCustomProvider.context_window() returns 200000
    And RhaiCustomProvider.max_output_tokens() returns 8192
    And no warning is logged about a missing "get_model_limits" function

  Scenario: Partial override — only context_window set, max_output_tokens falls back
    Given a custom Rhai provider with JSON ModelDef declaring context_window 128000 and max_output_tokens 4096
    And the Rhai script defines "fn get_model_limits(config) { #{ context_window: 400000 } }"
    When a RhaiCustomProvider is constructed for that model alias
    Then RhaiCustomProvider.context_window() returns 400000
    And RhaiCustomProvider.max_output_tokens() returns 4096

  Scenario: Invalid non-positive value is rejected and JSON ModelDef value is used
    Given a custom Rhai provider named "claude-rhai" with JSON ModelDef declaring context_window 128000
    And the Rhai script defines "fn get_model_limits(config) { #{ context_window: -1 } }"
    When a RhaiCustomProvider is constructed for that model alias
    Then RhaiCustomProvider.context_window() returns 128000
    And a warning is logged naming provider "claude-rhai" and key "context_window"

  Scenario: Script branches on config.model_alias to return alias-specific limits
    Given a custom Rhai provider named "claude-rhai" with JSON ModelDefs "opus-4.7" (128000) and "opus-4.6" (200000)
    And the Rhai script defines get_model_limits that returns 400000 for "opus-4.7" and 200000 for "opus-4.6"
    When a RhaiCustomProvider is constructed for model alias "opus-4.7"
    Then RhaiCustomProvider.context_window() returns 400000
    When a RhaiCustomProvider is constructed for model alias "opus-4.6"
    Then RhaiCustomProvider.context_window() returns 200000

  Scenario: Script sets an absolute-tokens compaction threshold
    Given a custom Rhai provider with JSON ModelDef declaring context_window 1000000
    And the Rhai script returns "#{ context_window: 400000, compaction_threshold: #{ type: \"tokens\", value: 200000 } }"
    When a RhaiCustomProvider is constructed
    Then RhaiCustomProvider.script_compaction_threshold() returns Some(("tokens", 200000))
    And the NAPI session-creation path calls ProviderManager.set_compaction_threshold_override with the same tuple
    And RhaiCustomProvider.context_window() returns 400000

  Scenario: Script sets a percentage compaction threshold
    Given a custom Rhai provider with JSON ModelDef declaring context_window 1000000
    And the Rhai script returns "#{ compaction_threshold: #{ type: \"percentage\", value: 75 } }"
    When a RhaiCustomProvider is constructed
    Then RhaiCustomProvider.script_compaction_threshold() returns Some(("percentage", 75))
    And RhaiCustomProvider.context_window() returns 1000000

  Scenario: Invalid compaction_threshold shape is rejected and no override is surfaced
    Given a custom Rhai provider named "claude-rhai" with JSON ModelDef declaring context_window 200000
    And the Rhai script returns "#{ compaction_threshold: #{ type: \"percentage\", value: 150 } }"
    When a RhaiCustomProvider is constructed
    Then RhaiCustomProvider.script_compaction_threshold() returns None
    And a warning is logged naming provider "claude-rhai" and key "compaction_threshold"
