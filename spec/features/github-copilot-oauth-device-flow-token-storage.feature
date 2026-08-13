@done
@rust
@credentials
@authentication
@providers
@PROV-054
Feature: GitHub Copilot OAuth device flow & token storage
  """
  Facade pattern ref: see claude_auth.rs and codex_auth.rs for credential persistence; codex_device_auth.rs is NOT reusable (different OAuth dialect)
  Module: rust/providers/src/copilot/{oauth.rs (device flow), auth.rs (credential persistence mirroring claude_auth.rs)} — part of larger copilot/ module built across PROV-054/055/056
  TUI integration requires new HookMode variants for deployment-type select and enterprise URL entry; existing oauth-device-waiting HookMode is reused for the polling phase
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. OAuth device flow uses GitHub OAuth App client_id Ov23li8tweQw6odWQebz with scope read:user (mirroring opencode copilot.ts:11)
  #   2. github.com deployments POST to https://github.com/login/device/code then poll https://github.com/login/oauth/access_token
  #   3. Enterprise deployments POST to https://<enterprise-domain>/login/device/code and poll https://<enterprise-domain>/login/oauth/access_token
  #   4. Login prompts for deploymentType (github.com or enterprise) and conditionally prompts for enterpriseUrl only when enterprise is selected
  #   5. Enterprise URL is normalized by stripping scheme and trailing slash (copilot.ts normalizeDomain line 15)
  #   6. On authorization_pending response, sleep for (interval + 3s safety margin) then poll again
  #   7. On slow_down response, increase polling interval by 5 seconds per RFC 8628 §3.5 (use server-provided interval if present)
  #   8. On access_token response, persist credential with refresh and access fields set to the same token value and expires set to 0 (never expires)
  #   9. Credential is persisted in auth.json under key github-copilot with file permissions 0600
  #
  # EXAMPLES:
  #   1. User runs `codelet auth login github-copilot` → CLI prompts deploymentType (github.com); device code returned; user enters code at https://github.com/login/device; polling loop succeeds; credential persisted at ~/.fspec/credentials/copilot_auth.json with mode 0600
  #   2. User runs `codelet auth login github-copilot` with deploymentType enterprise; CLI prompts for enterpriseUrl (ghe.example.com); device code flow completes against ghe.example.com; credential persisted with enterpriseUrl field set to ghe.example.com
  #   3. During polling, GitHub returns authorization_pending; polling loop sleeps for (interval + 3s) and retries until the user approves the device code
  #   4. During polling, GitHub returns slow_down with a higher interval; polling loop adopts the server-provided interval and adds a 5-second backoff per RFC 8628 §3.5
  #   5. User runs `codelet auth logout github-copilot`; credential file ~/.fspec/credentials/copilot_auth.json is deleted; next TUI open shows github-copilot as unauthenticated
  #
  # ========================================
  Background: User Story
    As a fspec user with a GitHub Copilot subscription
    I want to authenticate to GitHub Copilot via OAuth device flow from the CLI and have my credential persisted securely
    So that I can use my Copilot entitlement through fspec without re-entering credentials every session

  Scenario: Login to github.com Copilot deployment via OAuth device flow
    Given I have an active GitHub Copilot subscription on github.com
    And no existing github-copilot credential exists on disk
    When I run `codelet auth login github-copilot`
    And I select deploymentType "github.com" at the CLI prompt
    And the CLI displays a device code and the URL "https://github.com/login/device"
    And I enter the device code in my browser and approve the request
    Then the polling loop should exchange the device code for an access_token
    And a credential should be persisted at "~/.fspec/credentials/copilot_auth.json"
    And the credential file permissions should be 0600
    And the credential should contain access and refresh fields set to the same token value
    And the credential expires field should be 0

  Scenario: Login to GitHub Enterprise Copilot deployment with normalized enterprise URL
    Given I have an active GitHub Copilot subscription on a GitHub Enterprise instance
    And no existing github-copilot credential exists on disk
    When I run `codelet auth login github-copilot`
    And I select deploymentType "enterprise" at the CLI prompt
    And I enter "https://ghe.example.com/" at the enterpriseUrl prompt
    Then the enterprise URL should be normalized to "ghe.example.com" (scheme and trailing slash stripped)
    And the device code flow should POST to "https://ghe.example.com/login/device/code"
    And the polling loop should POST to "https://ghe.example.com/login/oauth/access_token"
    And a credential should be persisted with the enterpriseUrl field set to "ghe.example.com"

  Scenario: Polling loop handles authorization_pending by sleeping interval plus 3 second safety margin
    Given I have started a `codelet auth login github-copilot` session
    And the device code has been issued with a polling interval of 5 seconds
    When the polling endpoint returns "authorization_pending"
    Then the polling loop should sleep for 8 seconds (5 second interval + 3 second safety margin)
    And the polling loop should retry the access_token request
    And polling should continue until the user approves the device code or the code expires

  Scenario: Polling loop handles slow_down by increasing interval per RFC 8628 §3.5
    Given I have started a `codelet auth login github-copilot` session
    And the device code has been issued with a polling interval of 5 seconds
    When the polling endpoint returns "slow_down" with a server-provided interval of 10 seconds
    Then the polling loop should adopt the server-provided interval of 10 seconds
    And the polling loop should add a 5 second backoff per RFC 8628 §3.5
    And subsequent polls should use the new interval

  Scenario: Logout deletes the github-copilot credential file
    Given I am logged in to github-copilot with a credential at "~/.fspec/credentials/copilot_auth.json"
    When I run `codelet auth logout github-copilot`
    Then the file "~/.fspec/credentials/copilot_auth.json" should be deleted
    And opening the codelet TUI should show github-copilot as unauthenticated
