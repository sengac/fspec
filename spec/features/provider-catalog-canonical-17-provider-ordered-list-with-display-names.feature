@done
@RPC-107
@providers
@provider-settings
@rust
@ts-parity
Feature: Provider catalog: canonical 17-provider ordered list with display names
  """
  Rust enum/struct design: introduce a static const slice CANONICAL_PROVIDERS: &[CanonicalProvider] in rust/providers/src/catalog.rs (NEW file) carrying { id: &'static str, display_name: &'static str, env_var: &'static str, auth_type: AuthType, default_base_url: Option<&'static str> } for each of the 17 TS-canonical entries. AuthType is a Rust enum { ApiKey, OAuth } matching TS authType. Re-export via rust/providers/src/lib.rs `pub mod catalog;`. The list_provider_credentials code path consumes this slice to populate ProviderCredentialInfo display_name + ordering — single source of truth, no scattered match arms
  Cross-transport parity contract: extend rust/fspec-tui/tests/rpc054_cross_transport_parity.rs with a new scenario asserting that BOTH the embedded transport AND the websocket transport surface the same 17 canonical rows in the same canonical order with the same display_name strings. The CANONICAL_PROVIDERS slice lives BELOW the wire boundary (in codelet-providers, NOT in codelet-fspec-tui) so both transports see the same data without view-layer divergence. Coordinate with Agent E if a new RPC method list_canonical_providers() is needed for the TS frontend to converge onto the Rust catalog
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust catalog declares exactly 17 canonical providers in the same order as TS src/utils/provider-registry.ts SUPPORTED_PROVIDERS (L18-36): openai, anthropic, cohere, gemini, mistral, xai, together, huggingface, openrouter, groq, deepseek, moonshot, galadriel, azure, zai, codex, github-copilot
  #   2. Each catalog entry carries the TS-canonical display_name from PROVIDER_REGISTRY (provider-registry.ts L43-217): 'OpenAI API', 'Anthropic', 'Cohere', 'Google Gemini', 'Mistral AI', 'xAI', 'Together AI', 'Hugging Face', 'OpenRouter', 'Groq', 'DeepSeek', 'Moonshot', 'Galadriel', 'Azure OpenAI', 'Z.AI', 'Codex (ChatGPT)', 'GitHub Copilot' — bytes-for-bytes identical to TS
  #   3. The list_provider_credentials impl in rust/sessions/src/handle_impl.rs:869-895 returns ALL 17 canonical providers FIRST in canonical order (regardless of whether their env var is set), then appends any custom providers discovered on disk. The hard-coded 6-built-in loop in rust/providers/src/custom/management.rs:99-118 is REPLACED with iteration over CANONICAL_PROVIDERS
  #   4. The Rust slug for Anthropic is 'anthropic' (TS-canonical) NOT 'claude' (the current Rust internal slug at rust/providers/src/custom/management.rs:100). Any internal callers using 'claude' must be aliased or migrated — the wire/UX surface uses 'anthropic' exclusively
  #   5. ProviderCredentialInfo.display_name (rust/rpc-types/src/lib.rs:395) is populated from the canonical catalog NOT from the slug — the TUI list and title row use ONLY display_name, never provider_id, for human-facing strings
  #
  # EXAMPLES:
  #   1. User opens /provider against an empty workspace (no env vars set, no custom providers); the body shows 17 rows in order — first row 'OpenAI API', second 'Anthropic', third 'Cohere', ..., sixteenth 'Codex (ChatGPT)', seventeenth 'GitHub Copilot'. All rows show as unconfigured (·)
  #   2. User opens /provider with only ANTHROPIC_API_KEY set; the body still shows 17 rows in canonical order; the 'Anthropic' row (row 2) shows '✓ configured', all other 16 rows show '·'. The configured count in the title is 1
  #   3. User opens /provider with three env vars set (ANTHROPIC_API_KEY, GROQ_API_KEY, OPENROUTER_API_KEY) AND one custom provider 'my-vllm' on disk; the body shows 17 canonical rows + 1 custom row = 18 total. The 17 canonical rows appear FIRST in canonical order with Anthropic/Groq/OpenRouter marked '✓ configured'; 'my-vllm' appears LAST as the 18th row
  #   4. Cross-transport parity: list_provider_credentials called through embedded transport returns the same 17 canonical rows in the same canonical order with the same canonical display names as the same call through websocket transport (against the same StubSessionManagerHandle)
  #
  # ========================================
  Background: User Story
    As a Rust frontend user
    I want to open /provider against any fspec workspace
    So that see the same 17 canonical providers in the same order with the same display names as the TS Ink reference

  @unit
  @registry
  Scenario: CANONICAL_PROVIDERS slice declares exactly 17 entries in the TS-canonical order
    Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    When the slice is iterated in declaration order
    Then it yields exactly 17 entries
    And the provider ids in order are "openai", "anthropic", "cohere", "gemini", "mistral", "xai", "together", "huggingface", "openrouter", "groq", "deepseek", "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot"

  @unit
  @registry
  Scenario: CANONICAL_PROVIDERS display names match the TS PROVIDER_REGISTRY byte-for-byte
    Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    When the display_name field is read from each entry in order
    Then the display names in order are "OpenAI API", "Anthropic", "Cohere", "Google Gemini", "Mistral AI", "xAI", "Together AI", "Hugging Face", "OpenRouter", "Groq", "DeepSeek", "Moonshot", "Galadriel", "Azure OpenAI", "Z.AI", "Codex (ChatGPT)", "GitHub Copilot"

  @unit
  @registry
  Scenario: CANONICAL_PROVIDERS tags codex, anthropic, and github-copilot as OAuth auth_type
    Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    When the auth_type field is read from each entry
    Then the entries with id "anthropic", "codex", and "github-copilot" have auth_type AuthType::OAuth
    And every other entry has auth_type AuthType::ApiKey

  @integration
  @registry
  Scenario: Empty workspace returns 17 canonical rows in order with canonical display names
    Given no provider env vars are set in the process environment
    And no custom provider configs exist on disk
    When list_provider_credentials is called
    Then the response contains exactly 17 ProviderCredentialInfo entries
    And the entries appear in canonical order with provider_id "openai", "anthropic", "cohere", "gemini", "mistral", "xai", "together", "huggingface", "openrouter", "groq", "deepseek", "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot"
    And every entry has display_name set to the TS-canonical display string
    And every entry has configured == false

  @integration
  @registry
  Scenario: ANTHROPIC_API_KEY alone marks the Anthropic row configured under slug "anthropic"
    Given the env var ANTHROPIC_API_KEY is set to "sk-ant-test"
    And no other provider env vars are set
    And no custom provider configs exist on disk
    When list_provider_credentials is called
    Then the response contains exactly 17 entries in canonical order
    And the entry at index 1 has provider_id "anthropic" and display_name "Anthropic" and configured == true
    And no entry has provider_id "claude"
    And every other entry has configured == false

  @integration
  @registry
  Scenario: Canonical rows precede custom providers in the response
    Given the env var ANTHROPIC_API_KEY is set
    And the env var GROQ_API_KEY is set
    And the env var OPENROUTER_API_KEY is set
    And a custom provider config "my-vllm" exists on disk
    When list_provider_credentials is called
    Then the response contains exactly 18 entries
    And the first 17 entries are the canonical providers in canonical order
    And the entry at index 17 has provider_id "my-vllm"
    And the entries with provider_id "anthropic", "groq", and "openrouter" have configured == true

  @integration
  @registry
  Scenario: display_name on every canonical entry is sourced from the catalog not the slug
    Given no provider env vars are set in the process environment
    When list_provider_credentials is called
    Then for every canonical entry the display_name differs from the provider_id where the TS canon differs
    And the entry with provider_id "openai" has display_name "OpenAI API"
    And the entry with provider_id "gemini" has display_name "Google Gemini"
    And the entry with provider_id "github-copilot" has display_name "GitHub Copilot"
    And the entry with provider_id "azure" has display_name "Azure OpenAI"
    And the entry with provider_id "codex" has display_name "Codex (ChatGPT)"

  @integration
  @parity
  Scenario: Embedded and WebSocket transports surface the same 17 canonical rows in the same order
    Given a SharedFspecService backed by a StubSessionManagerHandle
    And both an EmbeddedFspecBackend and a WebSocketFspecBackend over that service
    When list_provider_credentials is called via the embedded transport
    And list_provider_credentials is called via the WebSocket transport
    Then both responses contain exactly 17 entries
    And both responses list the canonical provider_ids in identical canonical order
    And both responses list the canonical display_names in identical canonical order
