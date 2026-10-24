@cli
@rpc
@integration
@read-only
Feature: Compatibility read-only tools through the existing frontend
  The client offers a local stdio MCP interface using its existing RPC backend.
  Legacy daemons retain read-only tools without a daemon update or network listener.

  Scenario: Discover and call the read-only tools
    Given a service client connected over MCP to a legacy daemon
    When it lists tools and requests work units
    Then only health and work-unit reads are advertised
    And the selected read reaches the configured backend

  Scenario: Reject mutations and unexpected arguments
    Given a service client connected over MCP to a legacy daemon
    When it requests send_input or supplies unexpected arguments
    Then the request is rejected before reaching the backend

  Scenario: Contain backend failures
    Given a backend that fails or returns excessive output
    When the service client requests work units
    Then it receives a bounded tool error without backend diagnostics

  Scenario: Reject unprotected remote daemon URLs
    Given an explicitly configured daemon URL
    When MCP client mode validates that URL
    Then only plain WebSocket URLs with numeric loopback hosts and no credentials are accepted

  Scenario: Bound stalled daemon calls
    Given a backend that never finishes a read
    When the service client requests daemon health
    Then it receives a tool error after the deadline

  Scenario: Page a large workspace without exposing full descriptions
    Given work units whose complete response exceeds the output limit
    When the client requests successive bounded summary pages
    Then results are ordered by unique ID with a continuation cursor
    And invalid limits and cursors are rejected
    And descriptions are omitted and summary strings are bounded

  Scenario: Read complete details in explicit chunks
    Given a record with oversized Unicode text
    When the client follows detail offsets and revisions
    Then each response is valid bounded JSON with explicit partial indicators
    And concatenated text reconstructs the complete record
    And changed records reject continuation

  Scenario: Negotiate bounded daemon reads without probing legacy RPC methods
    Given a daemon supporting the read-only pagination transport
    When the MCP frontend connects
    Then the handshake advertises bounded reads and no unsolicited snapshots
    And work-unit pages and details are sliced before RPC serialization
    And a legacy daemon uses the explicitly marked full-list fallback
