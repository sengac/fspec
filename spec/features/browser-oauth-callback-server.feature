@done
@authentication
@providers
@http-server
@PROV-013
Feature: Browser OAuth HTTP Server for PKCE Callback
  """
  New file: rust/providers/src/codex/codex_oauth_server.rs - Hyper-based HTTP server for browser OAuth flow.
  Binds to port 1455, serves /auth/callback and /cancel routes. Directly calls existing codex_oauth.rs
  functions (generate_pkce, generate_state, build_authorize_url, validate_oauth_callback) and codex_auth.rs
  functions (write_codex_auth) in-process. No NAPI round-trips - the entire flow stays in Rust.
  New fn exchange_authorization_code() added to codex_oauth.rs for the code-for-tokens exchange.
  Dependencies: add hyper, hyper-util, and open crates to rust/providers/Cargo.toml.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Server listens on port 1455 specifically for OAuth (separate from the random-port attachment server) - dedicated OAuth server instance
  #   2. Callback validates state parameter using existing validate_oauth_callback() to prevent CSRF
  #   3. Server has a 5-minute timeout - if no callback received, server shuts down and flow fails gracefully
  #   4. After successful code exchange, tokens persisted to ~/.codex/auth.json using existing write_codex_auth() pattern
  #   5. Server serves HTML success/error pages using existing HTML_SUCCESS and html_error() from codex_oauth.rs
  #   6. Browser opens automatically to the authorize URL using the open crate
  #   7. Entire OAuth HTTP server implemented in Rust using hyper - keeps all auth logic in one process without NAPI round-trips
  #   8. Server directly calls existing codex_oauth.rs functions (generate_pkce, generate_state, build_authorize_url, validate_oauth_callback) and codex_auth.rs functions (write_codex_auth) in-process
  #   9. Token exchange uses reqwest POST to {CODEX_ISSUER}/oauth/token with grant_type=authorization_code, code, code_verifier, client_id, redirect_uri
  #   10. Browser opened via open crate (needs to be added to providers/Cargo.toml) - cross-platform URL launching
  #
  # EXAMPLES:
  #   1. User clicks login: server starts on 1455, PKCE generated, browser opens auth URL, user authorizes, callback receives code+state, state validated, code exchanged for tokens, tokens persisted, server shuts down
  #   2. Callback receives mismatched state parameter: server returns HTML error page with CSRF warning, no tokens persisted, server shuts down
  #   3. User doesn't complete authorization within 5 minutes: server times out, returns timeout error to caller, server shuts down cleanly
  #   4. Port 1455 is already in use: server startup fails gracefully with clear error message indicating port conflict
  #   5. Code exchange with auth.openai.com/oauth/token fails (network error): server returns error page, error propagated to caller
  #   6. Request to /cancel route: server aborts pending OAuth flow, returns cancel page, shuts down
  #
  # ========================================
  Background: User Story
    As a user with a ChatGPT Plus/Pro subscription
    I want to complete browser-based OAuth login via a local callback server
    So that I can authenticate with Codex without needing a pre-existing auth.json

  Scenario: Successful browser OAuth login with PKCE
    Given no existing Codex credentials are available
    When I initiate browser OAuth login
    Then the OAuth server should start on port 1455
    And a PKCE code verifier and S256 challenge should be generated
    And the browser should open to the authorize URL with PKCE parameters
    When the OAuth callback receives an authorization code with valid state
    Then the code should be exchanged for tokens via POST to the token endpoint
    And the account ID should be extracted from the token response JWT
    And the tokens should be persisted to auth.json with account_id
    And the OAuth server should shut down
    And the login function should return the OAuth tokens

  Scenario: OAuth callback rejects mismatched state parameter
    Given the OAuth server is running and waiting for callback
    And the expected state parameter is "expected-state-abc"
    When the callback receives a request with state "wrong-state-xyz"
    Then the server should return an HTML error page with CSRF warning
    And no tokens should be persisted
    And the OAuth server should shut down
    And the login function should return a CSRF error

  Scenario: OAuth login times out after 5 minutes
    Given the OAuth server is running and waiting for callback
    When 5 minutes elapse without receiving a callback
    Then the OAuth server should shut down cleanly
    And the login function should return a timeout error

  Scenario: Port 1455 already in use
    Given port 1455 is already occupied by another process
    When I initiate browser OAuth login
    Then the login should fail with a port conflict error
    And the error message should indicate port 1455 is in use

  Scenario: Token exchange fails due to network error
    Given the OAuth server is running and waiting for callback
    When the callback receives an authorization code with valid state
    And the token exchange POST to auth.openai.com/oauth/token fails
    Then the server should return an HTML error page
    And no tokens should be persisted
    And the login function should return the exchange error

  Scenario: User cancels OAuth flow via cancel route
    Given the OAuth server is running and waiting for callback
    When a request is made to the /cancel route
    Then the server should return a cancel confirmation page
    And the OAuth server should shut down
    And the login function should return a cancellation error

  Scenario: PKCE code verifier meets RFC 7636 requirements
    When a PKCE code pair is generated
    Then the verifier should be at least 43 characters long
    And the verifier should only contain unreserved URI characters
    And the challenge should be the Base64URL-encoded SHA-256 of the verifier
    And the challenge method should be "S256"

  Scenario: OAuth authorize URL contains all required parameters
    Given a PKCE code pair has been generated
    And a state parameter has been generated
    When the authorize URL is built
    Then the URL should start with "https://auth.openai.com/oauth/authorize"
    And the URL should contain the client_id "app_EMoamEEZ73f0CkXaXp7hrann"
    And the URL should contain the redirect_uri for port 1455
    And the URL should contain the PKCE code challenge
    And the URL should contain the state parameter

  Scenario: Authorization code exchanged for tokens at token endpoint
    Given a valid authorization code and PKCE verifier
    When the code is exchanged at the token endpoint
    Then a POST should be sent to "https://auth.openai.com/oauth/token"
    And the request should include grant_type "authorization_code"
    And the request should include the code, code_verifier, client_id, and redirect_uri
    And the response should contain access_token, id_token, and refresh_token
