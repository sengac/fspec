@done
@parity
@security
@credentials
@masked-display
@rpc
@providers
@provider-settings
@tui
@rust
@RPC-108
Feature: Provider settings: credential masking with prefix-aware mask helper and source tag
  """
  Wire-type extension design: ProviderCredentialInfo in codelet/rpc-types/src/lib.rs:391-401 gains two new fields keeping Option<String> shape because napi(object) does not support discriminated enums (existing credential_type at L397-399 uses the same convention). Backwards-compatible: older clients ignore None fields, serde defaults Option to None on missing JSON keys. The masking helper lives in codelet/providers/src/credentials.rs as `pub fn mask_api_key(&str) -> String` — single source of truth across all transports, called by the codelet-providers credential-detection path before the data crosses the wire
  Cross-transport parity contract: extend codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs so the same env-seeded ANTHROPIC_API_KEY surfaces the same masked_key 'sk-ant-••••mnop' AND the same source 'env' through BOTH embedded and websocket transports. Masking happens server-side inside codelet-providers BEFORE the wire boundary — never on the client — so the bytes of the raw key never leave the manager process. The view layer (codelet/fspec-tui/src/views/provider_settings/list.rs) just renders whatever masked_key + source comes off the wire
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Masking helper follows the TS contract at src/utils/credentials.ts:277-291: keys shorter than 12 chars return the literal '••••••••'; otherwise the helper matches one of 5 known prefixes (sk-ant-, sk-, gsk_, AIza, xai-) in that declaration order, falls back to first-6-chars when no prefix matches, then appends '••••••••' + last-4-chars
  #   2. Prefix matching order is significant: 'sk-ant-' MUST be tested BEFORE 'sk-' because both would match an Anthropic key 'sk-ant-XXXX'; the Rust port iterates the prefix slice in TS regex declaration order [sk-ant-, sk-, gsk_, AIza, xai-] and stops at the first match
  #   3. ProviderCredentialInfo (codelet/rpc-types/src/lib.rs:393-401) gains two new optional fields: masked_key: Option<String> populated via mask_api_key() when the credential is api-key+configured, None otherwise; source: Option<String> tagged with one of 'explicit' | 'file' | 'env' | 'dotenv' matching TS ProviderConfigResult.source at src/utils/credentials.ts:56-59
  #   4. Source tag is determined by credential-detection order in codelet/providers/src/credentials.rs mirroring src/utils/credentials.ts getProviderConfig L219-266: 'file' when the credential came from ~/.fspec/credentials/credentials.json (TS L232); 'env' when from process.env (TS L243); 'dotenv' when parsed from .env in cwd (TS L260); 'explicit' when caller-supplied
  #   5. OAuth-type providers (anthropic, codex, github-copilot) do NOT use mask_api_key — their masked_key field stays None and the TUI renders the literal 'OAuth' string from the view layer (mirrors TS useProviderSettingsState.ts:289, 302, 315). The wire ProviderCredentialInfo never carries OAuth token bytes — masking would not be reversible/safe
  #
  # EXAMPLES:
  #   1. User exports ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnop and opens /provider; the Anthropic row renders '✓ sk-ant-••••••••mnop [env]' — the key is masked between the recognised 'sk-ant-' prefix and its last 4 chars, tagged with the 'env' source
  #   2. User saves GROQ_API_KEY=gsk_test_1234567890abcdef to ~/.fspec/credentials/credentials.json via /provider edit form; the Groq row renders '✓ gsk_••••••••cdef [file]' — the file source tag confirms credentials.json is the active layer
  #   3. User has a key 'short' (5 chars) somehow loaded for a custom provider; the masked rendering shows '••••••••' (all dots) per the <12-char fallback — confirms no prefix-extraction leak when the key is too short to expose any chars safely
  #   4. User opens /provider against an OAuth-only provider (codex); the Codex row renders '✓ OAuth [oauth]' — the masked_key wire field is None and the view layer substitutes the literal 'OAuth' string
  #
  # ========================================
  Background: User Story
    As a Rust frontend user
    I want to open /provider against my fspec workspace with env-var credentials set
    So that see each configured row show a masked key like 'sk-ant-••••mnop [env]' that proves which credential layer the key came from without ever exposing the full secret

  Scenario: Anthropic sk-ant- key from env is masked with sk-ant- prefix and env source tag
    Given the api key string "sk-ant-api03-abcdefghijklmnop"
    When mask_api_key is called on the key
    Then the result is "sk-ant-••••••••mnop"

  Scenario: OpenAI sk- key is masked with sk- prefix
    Given the api key string "sk-test-1234567890abcdef"
    When mask_api_key is called on the key
    Then the result is "sk-••••••••cdef"

  Scenario: Groq gsk_ key is masked with gsk_ prefix
    Given the api key string "gsk_test_1234567890abcdef"
    When mask_api_key is called on the key
    Then the result is "gsk_••••••••cdef"

  Scenario: Gemini AIza key is masked with AIza prefix
    Given the api key string "AIzaSyABCDEFGH1234IJKLmnop"
    When mask_api_key is called on the key
    Then the result is "AIza••••••••mnop"

  Scenario: xAI xai- key is masked with xai- prefix
    Given the api key string "xai-test-1234567890abcdef"
    When mask_api_key is called on the key
    Then the result is "xai-••••••••cdef"

  Scenario: Key with no recognised prefix falls back to first six characters
    Given the api key string "pktest-abcdefghijklmnop"
    When mask_api_key is called on the key
    Then the result is "pktest••••••••mnop"

  Scenario: Short key under twelve characters renders all dots with no prefix leak
    Given the api key string "short"
    When mask_api_key is called on the key
    Then the result is "••••••••"

  Scenario: Prefix order precedence: sk-ant- is matched before sk-
    Given the api key string "sk-ant-1234567890abcdef"
    When mask_api_key is called on the key
    Then the result starts with "sk-ant-" not "sk-"

  Scenario: OAuth provider has masked_key None on the wire
    Given the codex OAuth credential is configured via codex_auth.json
    When list_provider_credentials is called
    Then the codex ProviderCredentialInfo has masked_key equal to None
    Then the codex ProviderCredentialInfo credential_type is "oauth"

  Scenario: Unconfigured provider has masked_key and source as None
    Given no environment variables are set for the openai provider
    When list_provider_credentials is called
    Then the openai ProviderCredentialInfo configured is false
    Then the openai ProviderCredentialInfo masked_key is None
    Then the openai ProviderCredentialInfo source is None

  Scenario: List provider credentials populates masked_key and source for api-key provider from env
    Given OPENAI_API_KEY is set to "sk-test-1234567890abcdef" in the environment
    When list_provider_credentials is called
    Then the openai ProviderCredentialInfo masked_key is Some("sk-••••••••cdef")
    Then the openai ProviderCredentialInfo source is Some("env")

  Scenario: Cross-transport parity: embedded and websocket surface identical masked_key and source
    Given ANTHROPIC_API_KEY is set to "sk-ant-api03-abcdefghijklmnop" in the environment
    When list_provider_credentials is called through both the embedded and websocket transports
    Then both transports return masked_key Some("sk-ant-••••••••mnop") and source Some("env") for the anthropic entry
