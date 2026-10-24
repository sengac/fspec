@done
@BUG-215
@phase-1
@agent-core
@context-management
@compaction
Feature: Compaction builder uses effective provider limits
  """
  Testing: Exercise real handlers, persisted originals and local HTTP fixtures through exploration-budget exhaustion and final serialized-request admission. Retain BUG-214 cancellation and BUG-211 fail-closed synthesis coverage.
  Implementation:
  - Both registrations share Builder::capture in rust/agent-loop/src/generate_compaction_handler.rs. Resolve context_window and max_output_tokens through the existing ProviderManager resolver; do not change raw accessor semantics for unrelated auxiliary callers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compaction captures the selected provider effective context capacity and output allowance, including provider defaults when optional metadata is absent.
  #   2. Explicit model limits and provider clamps remain authoritative; request output must not exceed its positive admitted reservation.
  #   3. Invalid limits and oversized synthesis remain fail-closed without dispatch or durable promotion.
  #
  # EXAMPLES:
  #   1. Both registered and GenerateCompaction routes complete final synthesis when output metadata is absent but the selected provider has an effective positive fallback.
  #   2. Both compaction routes use effective provider defaults when context and output metadata are both absent.
  #   3. Both compaction routes preserve explicit context and output limits when preparing final synthesis.
  #   4. Invalid explicit limits reject synthesis locally and leave durable context unchanged.
  #
  # ========================================
  Background: User Story
    As a developer using a profile-backed agent session
    I want to complete compaction with the selected provider effective limits
    So that missing optional model metadata does not prevent safe recovery

  Scenario: Missing output metadata uses the effective provider reservation
    Given a profile has explicit context capacity but no output metadata
    When both compaction routes exhaust exploration and synthesize a summary
    Then synthesis uses the provider output fallback and commits a valid summary

  Scenario: Missing context and output metadata use effective provider defaults
    Given a profile has neither context nor output metadata
    When both compaction routes exhaust exploration and synthesize a summary
    Then synthesis uses both provider defaults and commits a valid summary

  Scenario: Explicit limits and provider clamps remain authoritative
    Given profiles have explicit limits including overrides above a provider hard cap
    When both compaction routes exhaust exploration and synthesize a summary
    Then synthesis respects the resolved output allowance and commits a valid summary

  Scenario: Invalid explicit limits fail closed without durable promotion
    Given profiles have zero capacity or zero output or no remaining input budget
    When both compaction routes attempt compaction
    Then invalid limits are rejected locally and durable originals remain unchanged
