@done
@provider-settings
@tui
@ts-parity
@rust
@PROV-112
Feature: OAuth backend/RPC/transport surface + disconnect-oauth confirm flow
  """
  Wiring boundary (napi-direct via embedded transport, PROV-105 §8.2): new FspecBackend OAuth methods forward to codelet/napi/src/{claude,codex,copilot}_oauth.rs; websocket transport gets no-op/unsupported defaults. New rpc-types flow/result types; FspecService RPC methods; Action enum variants (OAuthDisconnect{provider_id}, ProviderCredentialsLoaded refresh). New ProviderSettingsMode::DisconnectOAuth{provider_id} replaces the dead-end DetailSub::OAuthNotice for the disconnect path. New dispatch_provider_settings_oauth.rs mirrors the PROV-109 dispatch_provider_settings_profiles.rs spawn→backend→ProviderCredentialsLoaded refresh loop. list_actions.rs routes Enter/d on OauthStatus to DisconnectOAuth (not open_delete_confirm). Disconnect clear is per-provider: anthropic deletes claude_auth.json, github-copilot deletes copilot_auth.json, codex removes only the tokens field (preserves OPENAI_API_KEY); all idempotent, errors swallowed without leaking RPC names. Files <300 LoC; clippy -D warnings + fmt clean; NO git; do not touch user WIP (main.rs, session_manager.rs) — hence napi-direct rather than through-core. Offline tests use a MockBackend implementing FspecBackend OAuth methods with call counters + scripted Ok/Err (mirrors PROV-109 provider_settings_profile_dispatch tests).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Enter or d/D on an oauth-status (Logout) row opens a dedicated DisconnectOAuth confirm dialog keyed by provider_id; it must NOT fall through to the generic api-key/delete-credentials confirm
  #   2. In the DisconnectOAuth confirm: 'y'/'Y' dispatches OAuthDisconnect{provider_id} then returns to list; 'n'/'N'/Esc returns to list without any backend call; any other key is consumed (no-op, stays in confirm)
  #   3. OAuthDisconnect dispatch calls the backend clear method for the provider exactly once (anthropic→clear claude tokens, github-copilot→clear copilot credential, codex/fallback→clear codex tokens preserving OPENAI_API_KEY), then re-fetches credentials → ProviderCredentialsLoaded refresh
  #   4. After a successful disconnect the nav reloads: the provider's oauth-status (Logout) row disappears (hasOAuthTokens=false) and the cursor returns to the parent provider row
  #   5. A backend clear error is swallowed: the dispatch emits a status WITHOUT leaking the RPC/method name and the UI silently returns to list (parity with PROV-109 error-path); clears are idempotent
  #   6. The FspecBackend OAuth methods are napi-direct on the embedded transport; the websocket transport provides no-op/unsupported defaults; browser login is unavailable off embedded (gating consumed by PROV-113/114)
  #
  # EXAMPLES:
  #   1. User with connected ChatGPT presses Enter on 'Logout from OAuth [ChatGPT]', presses 'y' → codex clear called once (OPENAI_API_KEY kept), nav reloads, Logout row gone, cursor on the Codex provider row
  #   2. User presses 'd' on 'Logout from OAuth [Claude]' → DisconnectOAuth confirm opens (not the api-key delete confirm); pressing Esc returns to list with claude tokens still present and the Logout row still showing
  #   3. User confirms disconnect for GitHub Copilot with 'y' but the backend clear errors → UI returns to list silently, no RPC/method name shown anywhere, and a subsequent reload reflects whatever state persists (idempotent)
  #   4. In the DisconnectOAuth confirm dialog the user presses 'x' (an unrelated key) → nothing happens, the confirm stays open and waits for y/n/Esc
  #
  # ========================================
  Background: User Story
    As a fspec-tui user with a connected OAuth provider
    I want to disconnect/logout from an OAuth provider in Provider Settings, backed by a real OAuth wiring boundary
    So that my tokens are actually cleared and the nav updates, instead of hitting the dead-end OAuthNotice placeholder

  @tui
  @provider-settings
  @oauth
  Scenario: Enter on an oauth-status row opens the DisconnectOAuth confirm
    Given a provider "anthropic" is expanded with OAuth tokens present
    And the cursor is on the "Logout from OAuth [Claude]" row
    When the user presses Enter
    Then the mode becomes DisconnectOAuth for provider "anthropic"
    And the generic api-key delete-credentials confirm is not opened
    And no backend clear call has been made yet

  @tui
  @provider-settings
  @oauth
  Scenario: Pressing d on an oauth-status row opens the DisconnectOAuth confirm not the api-key delete
    Given a provider "anthropic" is expanded with OAuth tokens present
    And the cursor is on the "Logout from OAuth [Claude]" row
    When the user presses "d"
    Then the mode becomes DisconnectOAuth for provider "anthropic"
    And the generic delete-credentials confirm is not opened

  @tui
  @provider-settings
  @oauth
  Scenario: Confirming disconnect clears codex tokens once and refreshes the nav
    Given a provider "codex" is expanded with OAuth tokens present
    And the cursor is on the "Logout from OAuth [ChatGPT]" row
    And the user has opened the DisconnectOAuth confirm
    When the user presses "y"
    Then the backend codex clear-tokens method is called exactly once
    And the cached OPENAI_API_KEY is preserved
    And the credentials are re-fetched producing a ProviderCredentialsLoaded refresh
    And the "Logout from OAuth [ChatGPT]" row is gone
    And the cursor returns to the "codex" provider row

  @tui
  @provider-settings
  @oauth
  Scenario: Cancelling disconnect with Esc preserves the tokens
    Given a provider "anthropic" is expanded with OAuth tokens present
    And the user has opened the DisconnectOAuth confirm
    When the user presses Esc
    Then no backend clear call is made
    And the mode returns to list
    And the "Logout from OAuth [Claude]" row is still shown

  @tui
  @provider-settings
  @oauth
  Scenario: Cancelling disconnect with n makes no backend call
    Given a provider "anthropic" is expanded with OAuth tokens present
    And the user has opened the DisconnectOAuth confirm
    When the user presses "n"
    Then no backend clear call is made
    And the mode returns to list

  @tui
  @provider-settings
  @oauth
  Scenario: An unrelated key in the confirm dialog is consumed and the dialog stays open
    Given a provider "anthropic" is expanded with OAuth tokens present
    And the user has opened the DisconnectOAuth confirm
    When the user presses "x"
    Then nothing happens
    And the mode is still DisconnectOAuth for provider "anthropic"
    And no backend clear call is made

  @tui
  @provider-settings
  @oauth
  @integration
  Scenario: A backend clear error returns to list silently without leaking the RPC name
    Given a provider "github-copilot" is expanded with OAuth tokens present
    And the backend clear-credential method is scripted to return an error
    And the user has opened the DisconnectOAuth confirm
    When the user presses "y"
    Then the UI returns to list
    And no RPC or method name is shown anywhere in the UI
    And the clear operation is idempotent on a subsequent reload

  @tui
  @provider-settings
  @oauth
  Scenario Outline: Disconnect routes to the correct per-provider clear method
    Given a provider "<provider>" is expanded with OAuth tokens present
    And the user has opened the DisconnectOAuth confirm
    When the user presses "y"
    Then the backend "<clear_method>" is called exactly once for provider "<provider>"

    Examples:
      | provider       | clear_method             |
      | anthropic      | claude clear-tokens      |
      | codex          | codex clear-tokens       |
      | github-copilot | copilot clear-credential |

  @tui
  @provider-settings
  @oauth
  @integration
  Scenario: OAuth backend methods are napi-direct on embedded and no-op on websocket
    Given the embedded transport is in use
    Then the FspecBackend OAuth methods forward to the napi OAuth functions
    When the websocket transport is in use
    Then the FspecBackend OAuth methods resolve to the unsupported/no-op defaults
