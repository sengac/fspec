@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex DNS reconnection
  """
  DNS lookup failures are typed connection failures on both selected transports. They enter the same cancellable offline wait as connection refusal without classifying permanent TLS or protocol failures as network outages. Resolution and connection remain inside the existing setup deadline.
  """

  Background: User Story
    As a Codex session owner
    I want DNS outages to wait for network reception
    So that losing reception before hostname resolution does not terminate my turn

  Scenario: DNS failures enter cancellable offline waiting on either transport
    Given a Codex session whose selected transport cannot resolve its hostname
    When the session attempts its original request
    Then it waits for network reception until cancellation without publishing a checkpoint
