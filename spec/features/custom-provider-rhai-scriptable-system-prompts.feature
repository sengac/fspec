@done
@configuration
@validator
@rust
@providers
@PROV-065
Feature: Custom provider Rhai-scriptable system prompts
  """
  RhaiSystemPromptFacade implements SystemPromptFacade trait in rust/providers/src/custom/system_prompt.rs; uses ScriptLoader+Engine from PROV-062; leaked strings for 'static lifetime; safe fallback defaults when optional Rhai functions are absent
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. RhaiSystemPromptFacade implements the SystemPromptFacade trait and delegates to 3 optional Rhai functions: identity_prefix, transform_preamble, format_system_prompt
  #   2. If identity_prefix(config) is defined it returns a String that becomes the facade's identity prefix; otherwise no prefix is applied
  #   3. If transform_preamble(config, preamble, fspec_guidance) is defined it returns the fully composed preamble; otherwise the default is fspec_guidance prepended to preamble
  #   4. If format_system_prompt(config, preamble, fspec_guidance) is defined it returns either a plain string (String variant) or a structured map with format:'array' and blocks for API output
  #   5. When format_system_prompt is not defined the default produces a plain JSON string containing fspec_guidance + preamble
  #   6. When format_system_prompt returns a map with format='array' the facade produces a JSON array of block objects preserving cache_control metadata
  #   7. All Rhai system prompt function invocations run inside tokio::task::spawn_blocking since facade methods are sync but must be safe from async contexts
  #   8. Rhai runtime errors during system prompt formatting surface as facade errors that are converted to the upstream ProviderError pathway and do not panic
  #   9. The identity_prefix() trait method returns Option<&'static str>; Rhai-returned strings are stored once via a OnceCell<&'static str> leak so the trait's static lifetime is upheld
  #   10. provider() trait method returns the custom provider's name as a 'static leaked str derived from ProviderConfig.name
  #
  # EXAMPLES:
  #   1. A script defining identity_prefix returning 'You are MyBot' yields facade.identity_prefix() == Some("You are MyBot")
  #   2. A script without identity_prefix yields facade.identity_prefix() == None
  #   3. With no transform_preamble defined, facade.transform_preamble('user text') returns FSPEC_WORKFLOW_GUIDANCE + '\n\n' + 'user text'
  #   4. A script defining transform_preamble that returns 'PREFIX: ' + preamble yields facade.transform_preamble('body') == 'PREFIX: body'
  #   5. A script without format_system_prompt produces facade.format_for_api('body') == JSON string '<fspec_guidance>\n\nbody'
  #   6. A script defining format_system_prompt returning #{format:'array', blocks:[#{type:'text', text:'prefix'}, #{type:'text', text:'body', cache_control:#{type:'ephemeral'}}]} produces a JSON array with the blocks and cache_control preserved
  #   7. A script defining format_system_prompt returning a plain string 'abc' produces facade.format_for_api('body') == JSON string 'abc'
  #   8. facade.provider() returns the custom provider's name string matching ProviderConfig.name
  #   9. A script throwing a runtime error from format_system_prompt causes facade.format_for_api to return a fallback JSON string rather than panicking
  #
  # ========================================
  Background: User Story
    As a custom provider author
    I want to define how my provider formats the system prompt (prefix text, preamble transformation, final API shape) via optional Rhai functions
    So that my provider can use either a plain string or a structured array with cache_control metadata without touching Rust code

  Scenario: Identity prefix from Rhai function
    Given a Rhai script that defines identity_prefix returning "You are MyBot"
    When I build a RhaiSystemPromptFacade from that script
    Then facade.identity_prefix() returns Some("You are MyBot")

  Scenario: Identity prefix defaults to None
    Given a Rhai script that does not define identity_prefix
    When I build a RhaiSystemPromptFacade from that script
    Then facade.identity_prefix() returns None

  Scenario: Default transform_preamble prepends fspec guidance
    Given a Rhai script that does not define transform_preamble
    When I call facade.transform_preamble("user text")
    Then the result equals FSPEC_WORKFLOW_GUIDANCE concatenated with two newlines and "user text"

  Scenario: Custom transform_preamble overrides default
    Given a Rhai script whose transform_preamble returns "PREFIX: " + preamble
    When I call facade.transform_preamble("body")
    Then the result equals "PREFIX: body"

  Scenario: Default format_for_api returns plain JSON string
    Given a Rhai script with no system prompt functions defined
    When I call facade.format_for_api("body")
    Then the result is a JSON String whose value starts with FSPEC_WORKFLOW_GUIDANCE and ends with "body"

  Scenario: format_system_prompt returning array produces JSON array with cache_control
    Given a Rhai script whose format_system_prompt returns a map with format "array" and two blocks including cache_control ephemeral on the second
    When I call facade.format_for_api("body")
    Then the result is a JSON array whose second block contains cache_control.type equal to "ephemeral"

  Scenario: format_system_prompt returning string produces plain JSON string
    Given a Rhai script whose format_system_prompt returns the plain string "abc"
    When I call facade.format_for_api("body")
    Then the result is a JSON String equal to "abc"

  Scenario: Facade reports custom provider name
    Given a ProviderConfig with name "my-llm"
    When I build a RhaiSystemPromptFacade from that config
    Then facade.provider() returns "my-llm"

  Scenario: Runtime error in format_system_prompt falls back gracefully
    Given a Rhai script whose format_system_prompt throws a runtime error
    When I call facade.format_for_api("body")
    Then the process does not panic and the result is a JSON String containing the default formatted preamble
