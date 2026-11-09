@done
@validation
@cli
@oauth
@provider-abstraction
@providers
@high
@PROV-168
@critical
Feature: Current Anthropic OAuth protocol
  """
  Protocol investigated against installed Claude Code 2.1.316 using fake credentials and a loopback capture, plus official thinking and effort documentation. OAuth version comes from claude --version or an explicit FSPEC_CLAUDE_CODE_VERSION deployment override; never use a bundled fallback. Enforce the observed supported baseline 2.1.280; retain server-side higher minimum errors. Preserve bearer authentication, Messages beta query, x-app cli, OAuth and Claude Code betas, model-aware legacy thinking, and Claude identity prefix. Adaptive thinking uses output_config.effort and summarized display. Optional SDK telemetry, billing attribution, context editing, per-turn control and extended cache TTL are not required for fspec features and must not be advertised without implementation. A pre-usage failure leaves occupancy unknown even with known capacity; no context percentage is invented.
  """

  Background: User Story
    As a Claude subscription user
    I want current models to receive a supported OAuth protocol and selected thinking effort
    So that requests succeed or provide actionable failure information

  Scenario: OAuth identity follows the installed release
    Given Claude Code is installed with a supported release
    When OAuth identity is resolved after the installed release changes
    Then the advertised version changes without rebuilding fspec

  Scenario: Explicit deployment version is used consistently
    Given an explicit supported Claude Code deployment version is configured
    When both OAuth providers and the reference headers are constructed
    Then all three use the configured identity without a bundled version

  Scenario: Unavailable or invalid versions fail closed
    Given the authoritative version source is unavailable malformed or older than the supported baseline
    When an OAuth provider is constructed
    Then construction fails with installation or update guidance and no fallback

  Scenario: API key authentication is independent of Claude Code
    Given Claude Code is unavailable and no deployment version is configured
    When an API key provider is constructed
    Then construction succeeds without an OAuth identity
