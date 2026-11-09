@BUG-472
@providers
@provider-abstraction
@PROV-152
Feature: Codex capability-aware request options
  """
  Only the Codex adapter resolves these options. Explicit user fields override
  advertised model defaults field by field. Unsupported optional defaults are
  omitted; explicit unsupported or malformed values return typed actionable
  errors before transport, never panic or silently downgrade.
  BUG-441: missing or stale capability data invents nothing: no default is
  applied and explicit options are sent unchanged for the Codex server to
  accept or reject (the former conservative fallback is removed; see
  codex-reasoning-for-every-codex-model.feature). Fresh capability snapshots
  apply on the next request; model names never imply capabilities.
  Capabilities are Codex-owned snapshots supplied by a trusted model source,
  not arbitrary request parameters. Credentials and headers are not options.
  MODEL-014: the trusted snapshot is the selected model's Codex /models entry
  (advertised efforts and default effort), set by ProviderManager::get_codex.
  A control the snapshot does not list (None) is not advertised and is sent
  unvalidated, as without a snapshot; an empty list still supports nothing.
  Provider updates reach already-created agents through shared model state.
  Codex always preserves store:false and reasoning.encrypted_content inclusion.
  Numeric effort strings advertised by a model serialize as unsigned numbers.
  Delivery uses stream_options.reasoning_summary_delivery=sequential_cutoff.
  Shared OpenAI enums and other providers' configuration remain unchanged.
  """

  Scenario: Advertised effort levels retain their wire representation
    Given fresh model capabilities advertise named and custom numeric effort levels
    When each supported effort is explicitly requested
    Then named efforts including xhigh serialize as strings
    And custom numeric efforts serialize as numbers without downgrade

  Scenario: Invalid explicit options fail with actionable typed errors
    Given a model advertises a restricted set of options
    When unsupported efforts malformed fields or unknown options are requested
    Then each request fails with an option-specific typed error before transport

  Scenario: Optional controls require advertised capabilities
    Given a model advertises context summary verbosity service tier and sequential-cutoff delivery
    When the user explicitly enables those controls
    Then each supported control appears in the request
    And the same controls are rejected by a model without those capabilities

  Scenario: Verbosity does not require a text format
    Given a model advertises text verbosity
    When the user configures only verbosity
    Then the request contains text verbosity without a fabricated format

  Scenario: Explicit configuration overrides supported model defaults
    Given model defaults include supported and unsupported optional controls
    When the user overrides one supported field
    Then the snapshot preserves other supported defaults and the explicit override
    And unsupported defaults are omitted while explicit unsupported values fail
    And required Codex fields cannot be overridden

  Scenario: Capability refresh affects the next request
    Given a capability snapshot does not support a requested effort
    When fresh capabilities advertise that effort
    Then the next request accepts the effort without changing explicit configuration

  Scenario: Arbitrary JSON option parsing never panics
    Given arbitrary nested JSON values and JSON text
    When Codex parses and resolves the options
    Then parsing returns resolved options or a typed error without panicking
