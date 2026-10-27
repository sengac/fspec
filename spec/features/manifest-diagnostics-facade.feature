@bug-fix
@session-management
@persistence
@phase-1
@BUG-274
Feature: Restore manifest diagnostics required by default RPC and Fspec builds

  """
  Existing staged BUG-271 RPC/UI calls predate BUG-273 and reference an absent core API. Prior six-library check/Clippy omitted RPC and the final binary. Supply one shared read-only manifest discovery path for session loading and diagnostics; keep target errors and unknown-reference cleanup strict. Preserve user staging and existing UI/RPC edits. Validate default binary, RPC test-fixture and Fspec test-stub-provider features.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Export a real non-draining diagnostic snapshot from core that is available to default RPC and Fspec builds; never mask the missing API with an empty stub or feature-only declaration.
  #   2. Diagnose individual unreadable manifests without changing bytes or hiding store-wide failures; keep healthy sessions available and damaged target loads and cleanup strict.
  #
  # EXAMPLES:
  #   1. Given broken and healthy manifest fixtures When the exported diagnostic API is called twice Then stable issues include paths identities categories recovery guidance and files stay unchanged
  #   2. Given an invalid sessions root When the exported diagnostic API is called Then a store-wide error is returned instead of an empty success
  #
  # ========================================

  Background: User Story
    As a developer building the default Fspec binary
    I want to compile and retrieve complete manifest diagnostics
    So that unreadable history is visible without breaking healthy sessions

  Scenario: Manifest diagnostics are stable and non destructive
    Given broken and healthy manifest fixtures
    When the exported diagnostic API is called twice
    Then stable issues include paths identities categories recovery guidance and files stay unchanged


  Scenario: Store wide diagnostic failures remain errors
    Given an invalid sessions root
    When the exported diagnostic API is called
    Then a store-wide error is returned instead of an empty success

