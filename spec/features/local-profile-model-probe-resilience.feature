@done
@PROV-131
@providers
@model-selection
@ts-parity
Feature: Local-profile model-probe resilience outside a tokio runtime
  """
  Fix location: codelet/sessions/src/profile_sections.rs::probe_profile_models — guard the block_in_place/Handle::current bridge with Handle::try_current()+runtime_flavor()==MultiThread; when absent, log via tracing::warn and return (Vec::new(), true) instead of panicking.
  No unwrap/expect/panic in the production path; the probe returns (discovered_ids, probe_failed) and callers already apply the MODEL-004 unreachable override in build_profile_provider_info.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. list_providers() must never panic, even when a local-server profile is configured and the /v1/models probe cannot run
  #   2. When no multi-thread tokio runtime is available, the local-profile /v1/models probe is skipped (not attempted) so block_in_place/Handle::current never panic
  #   3. A profile whose probe is skipped/fails degrades to an empty discovered-model list and is marked unreachable, but the section is still returned (parity with TS loadProfileSections try/catch)
  #
  # EXAMPLES:
  #   1. A local-server 'openai' profile is configured; list_providers() is called from a plain non-tokio context and returns without panicking
  #   2. The configured profile appears as a section with profile_name Some, an empty models list, and is_unreachable true when its probe was skipped
  #   3. On a multi-thread runtime the probe still runs normally (existing RPC-338 reachable/unreachable behaviour is preserved)
  #
  # ========================================
  Background: User Story
    As a codelet TUI user
    I want to open the model selector even when a configured local-server profile cannot be probed
    So that the app never crashes and the profile still appears (degraded) instead of taking down list_providers

  @server
  Scenario: list_providers does not panic when a local-server profile is configured and no tokio runtime is present
    Given a local-server "openai" profile named "spark-local" is configured
    And list_providers is called from a plain non-tokio context
    When the server builds the provider list
    Then list_providers returns without panicking

  @server
  Scenario: A profile whose probe is skipped degrades to an empty unreachable section
    Given a local-server "openai" profile named "spark-local" with no custom models is configured
    And list_providers is called from a plain non-tokio context so the /v1/models probe is skipped
    When the server builds the provider list
    Then the "spark-local" entry has profile_name Some("spark-local")
    And the "spark-local" entry has an empty models list
    And the "spark-local" entry has is_unreachable true
    And the "spark-local" entry is still present in the list

  @server
  Scenario: On a multi-thread runtime the local-server probe still runs
    Given a local-server "openai" profile named "spark-local" is configured
    And list_providers is called on a multi-thread tokio runtime
    When the server builds the provider list
    Then the "spark-local" entry is present in the list
    And list_providers returns without panicking
