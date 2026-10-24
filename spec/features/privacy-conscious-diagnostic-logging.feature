@BUG-239
@phase-1
@cli
@logging
@integration
@security
Feature: Privacy conscious diagnostic logging
  """
  Diagnostic formatting is a shared sink boundary for fspec file and stderr
  layers and RPC broadcast. Rig agent and provider spans are untrusted content
  sources, not authorization to persist payloads. FSPEC_LOG_PAYLOADS=1 is read
  at startup for local combined/client files only; RUST_LOG controls severity,
  not privacy. Payload fields are bounded to 4096 characters and credential
  fields plus recognizable bearer/key assignments are redacted. This cannot
  guarantee removal of secrets embedded in arbitrary prose. Directories are
  private and daily rotation retains five files per prefix. Existing historical
  logs are not purged. Conversation persistence is unchanged.
  """

  Background: User Story
    As an operator
    I want useful diagnostics without conversation payloads by default
    So that private conversations are not exposed in diagnostic bundles

  Scenario: Default local logs preserve errors without conversation payloads
    Given local diagnostic logging with ordinary trace verbosity
    When nested agent and tool spans emit synthetic payloads and an SSE failure
    Then local logs omit all system prompt completion and tool payload markers
    And correlation metadata and the complete actionable error chain remain readable

  Scenario: Explicit local payload diagnostics are bounded and credential safe
    Given explicit local payload diagnostics are enabled
    When nested agent and tool spans emit synthetic payloads and credentials
    Then local logs include bounded payload markers but omit credential values
    And local diagnostic storage is private with five file rotation retention

  Scenario: Broadcast never inherits local payload authorization
    Given explicit local payload diagnostics are enabled
    When a tool event with payload fields and credentials reaches RPC broadcast
    Then broadcast omits payload and credential markers
    And broadcast retains correlation metadata and actionable errors

  Scenario: Daemon stderr never inherits local payload authorization
    Given explicit local payload diagnostics are enabled
    When nested agent and tool spans emit synthetic payloads to daemon stderr
    Then stderr omits payload and credential markers
    And stderr retains correlation metadata and actionable errors
