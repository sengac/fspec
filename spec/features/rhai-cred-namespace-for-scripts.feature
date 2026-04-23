@done
@authentication
@rust
@security
@credentials
@oauth
@providers
@PROV-086
Feature: Add cred:: Rhai namespace exposing CredentialStore to scripts
  """
  Add pub fn build_cred_module(provider_name: String) -> RhaiModule in oauth/building_blocks.rs. Each registered native fn captures provider_name by clone and enforces name == provider_name before I/O.
  Add pub fn register_all_modules_for_provider(provider_name: &str) -> Vec<RhaiModule> that returns the default four modules plus build_cred_module(provider_name). register_all_modules() keeps its current no-cred signature for backward compatibility.
  Add pub fn build_provider_engine(provider_name: &str) -> Engine in oauth/engine.rs that calls build_sandboxed_engine(register_all_modules_for_provider(...)).
  Add pub fn fspec_home() -> PathBuf in oauth/building_blocks.rs mirroring claude_auth::get_fspec_home — honours FSPEC_HOME env, else $HOME/.fspec/credentials. Shared helper used by cred::path computation.
  cred::write uses std::fs::write + std::os::unix::fs::PermissionsExt to set_mode(0o600) on Unix. On non-Unix, permission setting is a no-op. Matches the sync-context approach of credential_store::write_sync + enforce_mode_0600.
  Name validation: return EvalAltResult::ErrorRuntime('cred:: access denied: <requested> does not match active provider <provider_name>') when name != provider_name. No path traversal escape since the name is rejected outright before any PathBuf construction.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A cred:: Rhai namespace is registered in the default sandboxed engine alongside http::, crypto::, json::, and oauth::
  #   2. cred::read(name) returns a Map when the credential file exists and parses as JSON, or unit () when it does not exist
  #   3. cred::write(name, map) persists the map as JSON to ~/.fspec/credentials/<name>.json with 0600 permissions on Unix
  #   4. cred::delete(name) removes the credential file and is idempotent — it succeeds when the file is absent
  #   5. cred::path(name) returns the absolute string path to the credential file without touching the filesystem
  #   6. The cred:: module is scoped to the active provider name — the provider name is bound at module build time and the name argument is used only as a filename component, never as a way to escape the active provider's credential file
  #   7. All credential I/O (read, write, delete) runs inside tokio::task::spawn_blocking when invoked from async host code; from Rhai scripts the calls are synchronous and safe because they use std::fs with Unix permission setting via std::os::unix::fs::PermissionsExt
  #   8. Path resolution for credential files honours FSPEC_HOME when set, otherwise falls back to $HOME/.fspec/credentials, matching claude_auth.rs::get_fspec_home()
  #
  # EXAMPLES:
  #   1. Script for provider 'acme' calls cred::write('acme', #{access_token: 'abc', refresh_token: 'def'}) — file ~/.fspec/credentials/acme.json is created with 0600 permissions and contains the JSON map
  #   2. Script for provider 'acme' calls cred::read('acme') after writing — receives a Map with access_token and refresh_token fields
  #   3. Script for provider 'acme' calls cred::read('acme') before writing — receives unit () indicating no credentials exist yet
  #   4. Script for provider 'acme' attempts cred::read('other_provider_auth') — the call returns an error because the requested name does not match the active provider 'acme'
  #   5. Script for provider 'acme' attempts cred::write('../../etc/passwd', #{}) — the call returns an error because the name does not match the active provider
  #   6. Script for provider 'acme' calls cred::path('acme') — receives the absolute path string ending in credentials/acme.json based on FSPEC_HOME
  #   7. Script for provider 'acme' calls cred::delete('acme') on a non-existent file — the call succeeds (idempotent) and returns unit
  #   8. build_default_engine() (which has no bound provider) does NOT register a cred:: module — scripts compiled against the default engine cannot access credential functions
  #   9. build_provider_engine('acme') registers the cred:: module bound to provider name 'acme' in addition to http::, crypto::, json::, oauth::
  #
  # ========================================
  Background: User Story
    As a Rhai OAuth script author
    I want to read, write, and delete provider-scoped credentials via a cred:: namespace
    So that my script can persist OAuth tokens without the host exposing arbitrary filesystem access

  Scenario: cred::write persists a map as JSON with 0600 permissions
    Given FSPEC_HOME is set to a temporary directory
    And a provider engine bound to provider name "acme"
    When a script calls cred::write("acme", #{access_token: "abc", refresh_token: "def"})
    Then the file acme.json is created under FSPEC_HOME with 0600 permissions on Unix
    And the file parses as a JSON object containing access_token and refresh_token

  Scenario: cred::read returns the written map
    Given FSPEC_HOME is set to a temporary directory
    And a provider engine bound to provider name "acme"
    And a script has previously called cred::write("acme", #{access_token: "abc"})
    When a script calls cred::read("acme")
    Then the script receives a Map whose access_token equals "abc"

  Scenario: cred::read returns unit when the credential file is absent
    Given FSPEC_HOME is set to a temporary directory
    And a provider engine bound to provider name "acme"
    And no credential file exists for "acme"
    When a script calls cred::read("acme")
    Then the script receives unit ()

  Scenario: cred::read rejects a name that does not match the active provider
    Given a provider engine bound to provider name "acme"
    When a script calls cred::read("other_provider_auth")
    Then the engine returns a runtime error mentioning access denied and the active provider name

  Scenario: cred::write rejects a path-traversal name
    Given a provider engine bound to provider name "acme"
    When a script calls cred::write("../../etc/passwd", #{})
    Then the engine returns a runtime error and no file is written outside FSPEC_HOME

  Scenario: cred::path returns the absolute credential path without touching the filesystem
    Given FSPEC_HOME is set to a temporary directory
    And a provider engine bound to provider name "acme"
    When a script calls cred::path("acme")
    Then the script receives a string ending with "acme.json" inside FSPEC_HOME
    And no file is created on disk

  Scenario: cred::delete is idempotent when the credential file is absent
    Given FSPEC_HOME is set to a temporary directory
    And a provider engine bound to provider name "acme"
    And no credential file exists for "acme"
    When a script calls cred::delete("acme")
    Then the call succeeds without error

  Scenario: default engine does not expose the cred namespace
    Given an engine built via build_default_engine()
    When a script attempts to call cred::path("anything")
    Then the engine returns an error because the cred module is not registered

  Scenario: provider engine registers cred alongside the other building block modules
    Given an engine built via build_provider_engine("acme")
    When a script calls oauth::generate_state(), crypto::sha256("x"), json::parse("{}"), and cred::path("acme") in sequence
    Then each call succeeds, confirming http, crypto, json, oauth, and cred modules are all registered
