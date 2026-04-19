@done
@validator
@validation
@config
@rust
@providers
@PROV-062
Feature: Provider config loader and Rhai script compiler

  """
  ProviderConfig is a serde-deserialized Rust struct; ScriptLoader caches Arc<rhai::AST> keyed by absolute path + mtime; discovery scans ~/.fspec/providers and .fspec/providers with project-local override by name; reuses build_sandboxed_engine from PROV-060 for compilation and required-function validation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Provider config JSON files are discovered from ~/.fspec/providers/*.json (global) and .fspec/providers/*.json (project-local)
  #   2. Project-local configs override global configs with the same 'name' field
  #   3. ProviderConfig is deserialized from JSON via serde with fields: name, display_name, base_url, script, auth, models, defaults, system_prompt, tool_style, api_style, headers, env_prefix
  #   4. AuthConfig is a tagged enum with variants: bearer, api_key_header, oauth_device_code, oauth_pkce, custom
  #   5. Provider name must not collide with built-in names (claude, openai, codex, gemini, zai, github-copilot, copilot)
  #   6. Provider name must match pattern ^[a-z][a-z0-9-]*$
  #   7. Script path is resolved relative to the config file's directory and must exist at load time
  #   8. Default model name (when specified in defaults) must exist as a key in the models map
  #   9. ScriptLoader compiles each .rhai file to a Rhai AST exactly once and caches by absolute path + mtime
  #   10. Compiled script AST is returned wrapped in Arc<AST> so it can be shared across provider instances without reparsing
  #   11. Rhai compilation uses the sandboxed engine factory from PROV-060 (build_sandboxed_engine) so scripts have access to registered building-block modules
  #   12. Syntax errors in .rhai scripts are reported with file path, line, and column from the underlying Rhai ParseError
  #   13. Script validation verifies the 7 required functions (build_request, build_headers, build_url, parse_response, parse_stream_chunk, build_stream_request, map_error) are defined after compilation
  #   14. FSPEC_HOME environment variable redirects the base directory used for locating ~/.fspec/providers/
  #
  # EXAMPLES:
  #   1. A config JSON with all required fields (name, display_name, base_url, script, auth, models) parses successfully and exposes a populated ProviderConfig
  #   2. A config JSON missing 'name' returns a serde error mentioning the missing field
  #   3. A config with name='claude' is rejected with a 'conflicts with built-in provider' error
  #   4. A config with name='My Provider' (invalid chars) is rejected with a pattern validation error
  #   5. A config references a script that doesn't exist -> load-time error naming the resolved script path
  #   6. A config with defaults.model='fast' but no 'fast' key in models -> load-time error about missing default model
  #   7. discover_provider_configs() with one global config and one project-local config of the same name returns only the project-local one
  #   8. discover_provider_configs() with no providers directories returns an empty Vec
  #   9. ScriptLoader.load() called twice on the same unchanged .rhai file parses it only once (second call hits cache)
  #   10. ScriptLoader.load() on a .rhai file that has been modified re-parses and updates the cache
  #   11. A .rhai file with syntax error 'fn build_request( { ' produces an error containing line/column and file path
  #   12. A .rhai file missing parse_response function fails the required-functions check with a clear message listing which function is missing
  #   13. A compiled script from ScriptLoader can successfully call the registered oauth::generate_pkce building block function from PROV-060
  #   14. auth.type='bearer' with env_var='MY_KEY' deserializes into AuthConfig::Bearer { env_var:'MY_KEY', token_prefix:'Bearer' }
  #   15. auth.type='oauth_device_code' with all required fields deserializes into AuthConfig::OauthDeviceCode
  #
  # ========================================

  Background: User Story
    As a provider plugin developer
    I want to place a JSON config + Rhai script in .fspec/providers/ and have fspec discover, validate, and compile them at load time
    So that I can add LLM providers without recompiling fspec and receive clear errors up front

  Scenario: Load a complete custom provider config JSON
    Given a JSON file containing name, display_name, base_url, script, auth, and models fields
    When I call ProviderConfig::from_file on that JSON path
    Then I get a ProviderConfig whose fields match the JSON values


  Scenario: Reject config JSON missing the required name field
    Given a JSON file that omits the name field
    When I load the config
    Then I receive an error whose message mentions the missing name field


  Scenario: Reject provider name that collides with a built-in provider
    Given a config JSON with name set to "claude"
    When I load the config
    Then I receive an error mentioning that the name conflicts with a built-in provider


  Scenario: Reject provider name with invalid characters
    Given a config JSON with name set to "My Provider"
    When I load the config
    Then I receive an error mentioning the allowed pattern ^[a-z][a-z0-9-]*$


  Scenario: Reject config when referenced script file does not exist
    Given a config JSON whose script field points to a nonexistent .rhai file
    When I load the config
    Then I receive an error including the resolved absolute script path


  Scenario: Reject config when default model is not present in models map
    Given a config JSON with defaults.model set to "fast" and models containing only "smart"
    When I load the config
    Then I receive an error mentioning the missing default model "fast"


  Scenario: Project-local config overrides global config with same name
    Given a global config ~/.fspec/providers/my-llm.json and a project-local .fspec/providers/my-llm.json both named "my-llm"
    When I call discover_provider_configs
    Then the returned list contains exactly one config for "my-llm" and it matches the project-local JSON


  Scenario: Return empty result when no providers directories exist
    Given neither ~/.fspec/providers/ nor .fspec/providers/ exists
    When I call discover_provider_configs
    Then I receive an empty Vec without error


  Scenario: ScriptLoader caches AST for unchanged script
    Given a valid .rhai file on disk that has not been modified between loads
    When I call ScriptLoader::load on the same path twice
    Then both calls return the same Arc<AST> instance and parsing occurs only once


  Scenario: ScriptLoader re-parses script when mtime changes
    Given a .rhai file that has been loaded once
    When I modify the file so its mtime advances and call ScriptLoader::load again
    Then a new Arc<AST> is returned reflecting the updated script content


  Scenario: Report Rhai syntax errors with file path line and column
    Given a .rhai file containing a syntactically invalid function declaration
    When I call ScriptLoader::load on that file
    Then the returned error includes the file path and the line and column from the Rhai ParseError


  Scenario: Reject script missing a required function
    Given a .rhai file that parses but does not define parse_response
    When I validate the compiled script against the required functions list
    Then I receive an error naming parse_response as the missing function


  Scenario: Compiled script can call registered PROV-060 building blocks
    Given a .rhai file that calls oauth::generate_pkce inside a function
    When I compile it with the shared sandboxed engine and execute that function
    Then the script runs successfully and returns a PKCE pair


  Scenario: Bearer auth config deserializes with default token prefix
    Given a config JSON with auth.type set to "bearer" and auth.env_var set to "MY_KEY"
    When I load the config
    Then the auth field is AuthConfig::Bearer with env_var "MY_KEY" and token_prefix "Bearer"


  Scenario: OAuth device code auth config deserializes with all fields
    Given a config JSON with auth.type set to "oauth_device_code" and client_id, device_code_url, token_url, credential_file all provided
    When I load the config
    Then the auth field is AuthConfig::OauthDeviceCode with matching fields

