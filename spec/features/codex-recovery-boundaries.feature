@phase-1
@providers
@cli
@provider-abstraction
@regression
Feature: Preserve Codex recovery at credential and committed-tool boundaries
  """
  Architecture notes:
  Credential connection failures retain the provider-owned offline recovery
  classification without retrying rejected or malformed credentials. Completed
  checkpoint calls do not need executable dialect normalization on interruption.
  A committed tool response finishes transport recovery before local tools run.
  """

  Background: User Story
    As a Codex session owner
    I want recovery to respect credential and tool completion boundaries
    So that temporary outages and interruption preserve completed work

  Scenario: Expired OAuth credentials wait cancellably for an offline issuer on either transport
    Given expired valid Codex credentials and an unavailable credential issuer
    When the selected transport attempts the original request
    Then the session waits for network reception without inference or added user input
    And cancelling the request clears offline recovery promptly

  Scenario: Rejected or malformed refreshed credentials never enter offline waiting
    Given expired Codex credentials whose issuer rejects refresh or returns malformed credentials
    When the session attempts its original request
    Then the credential failure ends the turn without recovery or inference requests

  Scenario: Interrupting restored completed dialect calls does not require a registry
    Given a restored checkpoint with completed custom and namespaced calls
    And their outcomes exist in the checkpoint or conversation
    When I interrupt before the first request prepares a tool registry
    Then interruption and completion are published without new tool outcomes or inference
    And existing call identities and outcomes remain unchanged

  Scenario: Committed tools finish transport recovery before local execution
    Given an ordinary or reconnected Codex response commits a tool call
    When the consumer receives that call and stops before the final stream event
    Then transport recovery has already succeeded without a cancellation notification
