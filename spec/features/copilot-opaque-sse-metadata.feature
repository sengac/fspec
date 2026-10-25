@done
@BUG-268
@providers
@provider-abstraction
Feature: Copilot opaque SSE metadata
  """
  The Copilot-only opaque capture removes private data before shared adapters.
  Preserve all ordinary SSE metadata without introducing inference retry policy.
  """

  Background: User Story
    As a Copilot streaming consumer
    I want transport event metadata preserved during private opaque capture
    So that ordinary adapter behavior remains unchanged

  Scenario: SSE metadata survives Copilot only opaque capture
    Given a synthetic Copilot response event carries an identifier event name retry directive and owned opaque
    When the exact production capture transforms its byte stream
    Then downstream event metadata remains unchanged and opaque is absent from its event data
