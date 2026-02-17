@session-management
@rust
@napi
@credentials
@CONFIG-005
Feature: Move Credential Management to Rust
  """
  Create codelet/napi/src/credentials/ module with: mod.rs, store.rs, types.rs, resolver.rs, napi_bindings.rs
  CredentialStore uses lazy_static global singleton with Mutex, matching persistence module pattern
  NAPI bindings: credentials_resolve(provider_id, project?) and credentials_reload()
  Remove api_key param from sessionManagerCreateWithId - Rust resolves internally
  ClaudeProvider::detect_auth_mode_from_token() checks token prefix: 'sk-ant-oat' -> OAuth (Bearer auth), anything else -> ApiKey (x-api-key header)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust must be the single source of truth for credential resolution - TypeScript only saves/deletes credentials
  #   2. Credential priority chain: 1) credentials file (~/.fspec/credentials/credentials.json), 2) environment variable, 3) .env file in project directory
  #   3. Credentials must be re-resolved on session resume to pick up any changes made while the session was inactive
  #   4. CredentialStore must use mtime-based change detection to reload credentials file only when it has changed
  #   5. The api_key parameter must be removed from sessionManagerCreateWithId NAPI function
  #   6. TypeScript must call credentials_reload() NAPI function after saving credentials to ensure Rust picks up changes immediately
  #   7. Credentials must never be returned to TypeScript via NAPI - they stay in Rust
  #   14. Auth mode must be determined by token PREFIX (sk-ant-oat = OAuth, sk-ant-api = API key), NOT by which environment variable contains the credential
  #
  # EXAMPLES:
  #   1. Session created with model 'anthropic/claude-sonnet-4-20250514' -> Rust extracts provider 'anthropic' -> resolves ANTHROPIC_API_KEY
  #   2. User updates API key in TUI credential dialog -> Rust session automatically uses new key on next API call without restart
  #   3. Credential resolution checks file first -> finds key in credentials.json -> uses that key
  #   4. Credential resolution checks file first -> no key in file -> checks ANTHROPIC_API_KEY env var -> uses that key
  #   5. Credential resolution checks file and env -> both empty -> checks project .env file -> finds key -> uses that key
  #   6. CredentialStore checks mtime -> file unchanged since last load -> returns cached credentials without disk read
  #   7. CredentialStore checks mtime -> file mtime changed -> reloads from disk and updates cache
  #   8. Session resumes after being inactive -> Rust re-resolves credentials -> picks up new API key that was saved while session was inactive
  #   9. TypeScript calls saveCredential() -> writes to file -> calls credentials_reload() NAPI -> Rust reloads cache
  #   17. OAuth token (sk-ant-oat01-...) stored in ANTHROPIC_API_KEY env var -> ClaudeProvider detects 'sk-ant-oat' prefix -> uses AuthMode::OAuth -> sends Authorization: Bearer header
  #   18. Standard API key (sk-ant-api-...) stored in ANTHROPIC_API_KEY env var -> ClaudeProvider detects non-OAuth prefix -> uses AuthMode::ApiKey -> sends x-api-key header
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to update my API credentials
    So that existing Rust sessions automatically pick up the new credentials without requiring a session restart

  # Priority Chain Tests (Rules 1, 2)
  @unit
  Scenario: Resolve credential from credentials file
    Given a credentials file exists with an API key for provider "anthropic"
    And no ANTHROPIC_API_KEY environment variable is set
    When credential resolution is requested for provider "anthropic"
    Then the API key from the credentials file should be returned

  @unit
  Scenario: Resolve credential from environment variable when file has no key
    Given no API key exists in the credentials file for provider "anthropic"
    And the ANTHROPIC_API_KEY environment variable is set
    When credential resolution is requested for provider "anthropic"
    Then the API key from the environment variable should be returned

  @unit
  Scenario: Resolve credential from project .env file as fallback
    Given no API key exists in the credentials file for provider "anthropic"
    And no ANTHROPIC_API_KEY environment variable is set
    And a .env file exists in the project directory with ANTHROPIC_API_KEY
    When credential resolution is requested for provider "anthropic" with project path
    Then the API key from the .env file should be returned

  @unit
  Scenario: Return no credential when no source has the key
    Given no API key exists in any credential source for provider "anthropic"
    When credential resolution is requested for provider "anthropic"
    Then no API key should be returned

  # Provider Extraction Tests
  @unit
  Scenario: Extract provider from model string
    Given a model string "anthropic/claude-sonnet-4-20250514"
    When a session is created with this model
    Then the provider "anthropic" should be extracted
    And credential resolution should use "anthropic" as the provider ID

  # Mtime-based Caching Tests (Rule 4)
  @unit
  Scenario: Cache credentials when file unchanged
    Given the CredentialStore has loaded credentials from disk
    And the credentials file mtime has not changed
    When credential resolution is requested
    Then the cached credentials should be returned without reading disk

  @unit
  Scenario: Reload credentials when file mtime changes
    Given the CredentialStore has loaded credentials from disk
    And the credentials file is modified with a new API key
    When credential resolution is requested
    Then the credentials should be reloaded from disk
    And the new API key should be returned

  # Session Resume Tests (Rule 3)
  @integration
  Scenario: Session resume picks up credential changes
    Given a Rust session exists with provider "anthropic"
    And the session was created with API key "old-key"
    And the credentials file is updated with API key "new-key"
    When the session is resumed
    Then credential resolution should be re-executed
    And the new API key "new-key" should be used

  # TypeScript Coordination Tests (Rules 5, 6)
  @integration
  Scenario: TypeScript saveCredential triggers Rust reload
    Given TypeScript saves a new API key to the credentials file
    When credentials_reload() NAPI function is called
    Then the CredentialStore should reload from disk
    And subsequent credential resolutions should return the new key

  # Session Creation Without API Key Parameter (Rule 5)
  @integration
  Scenario: Session creation resolves credentials internally
    Given a credentials file exists with an API key for provider "anthropic"
    When sessionManagerCreateWithId is called without an api_key parameter
    Then Rust should resolve the credential internally
    And the session should be created with the resolved API key

  # Security Test (Rule 7)
  @unit
  Scenario: Credentials never returned to TypeScript via NAPI
    Given credentials_resolve NAPI function exists
    When the API checks for functions that return credentials to TypeScript
    Then no NAPI function should return the actual API key value

  # OAuth Token Detection by Prefix (Rule 14)
  @integration
  @unit
  Scenario: Detect OAuth token from prefix and use Bearer authentication
    Given a credential with value "sk-ant-oat01-abc123" is available
    When ClaudeProvider initializes with this credential
    Then the auth mode should be detected as OAuth from the "sk-ant-oat" prefix
    And the Authorization header should use Bearer token format

  @unit
  Scenario: Detect API key from prefix and use x-api-key authentication
    Given a credential with value "sk-ant-api03-xyz789" is available
    When ClaudeProvider initializes with this credential
    Then the auth mode should be detected as ApiKey (non-OAuth prefix)
    And the x-api-key header should be used

  @integration
  Scenario: OAuth token in ANTHROPIC_API_KEY env var uses correct auth mode
    Given an OAuth token "sk-ant-oat01-test123" is stored in credentials.json
    And the credential resolver sets ANTHROPIC_API_KEY environment variable
    When a Claude session is created
    Then ClaudeProvider should detect OAuth mode from the token prefix
    And the session should authenticate using Authorization: Bearer header
    And the session should NOT use x-api-key header

  @integration
  Scenario: Standard API key in ANTHROPIC_API_KEY env var uses correct auth mode
    Given a standard API key "sk-ant-api03-standard456" is stored in credentials.json
    And the credential resolver sets ANTHROPIC_API_KEY environment variable
    When a Claude session is created
    Then ClaudeProvider should detect ApiKey mode from the token prefix
    And the session should authenticate using x-api-key header
