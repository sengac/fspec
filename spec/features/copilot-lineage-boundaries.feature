@done
@BUG-268
@providers
@session-management
Feature: Copilot lineage boundaries
  """
  Preserve the approved nonpersistent live ProviderSession lineage contract.
  These unchanged scenarios are separated from genuine-turn acceptance solely
  to retain native one-feature-to-one-test-file traceability.
  """

  Background: User Story
    As a Copilot user
    I want opaque metadata bound to compatible live ownership
    So that failures and provider changes cannot leak another request context

  Scenario: Cancelled or failed requests invalidate prior owned lineage
    Given a successful Copilot request owns opaque for an exact typed history
    When its next real request fails or is cancelled before settlement
    Then reconstructing the same session cannot replay the former opaque

  Scenario: Late superseded settlement cannot overwrite or clear newer lineage
    Given an older Copilot request is still in flight
    When a newer request settles before that older request succeeds fails or is cancelled
    Then the following compatible request retains only the newer opaque

  Scenario: Opaque remains private and caller supplied values are rejected
    Given a successful Copilot request owns opaque for an exact typed history
    When diagnostics serialization writers and caller supplied opaque are exercised
    Then only the transport sends owned opaque and no diagnostic or checkpoint adopts it

  Scenario: Actual provider selection invalidates Copilot without changing Codex state
    Given a successful Copilot request owns opaque for an exact typed history
    When a read only provider lookup or an actual provider selection occurs
    Then lookup preserves lineage but selection invalidates Copilot without resetting another extension

  Scenario: Deployment identity remains distinct at the same effective API origin
    Given a successful Copilot request owns opaque for a public GitHub deployment
    When an enterprise deployment reconstructs the same model at the same loopback API origin
    Then the real request cannot reuse public deployment opaque
