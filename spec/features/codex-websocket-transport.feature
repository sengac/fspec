@PROV-156
@CMPCT-081
@PROV-160
@providers
@streaming
@integration
@provider-abstraction
Feature: Codex optional WebSocket transport
  """
  CMPCT-081 supersedes PROV-156 incremental continuation and unsupported-upgrade
  fallback: explicitly selected WebSocket requests always send full current DAG input
  without previous_response_id and never downgrade to HTTP SSE.
  PROV-160 supersedes the accepted-disconnect no-replacement criterion with bounded
  same-transport recovery. Authentication and rejected upgrades remain terminal.
  Explicitly enabled Codex adapters own a reusable Responses socket. HTTP SSE remains
  the default. Cancellation retires the socket with a bounded close handshake.
  Routing and authorization are bound by the existing Codex HTTP middleware.
  """

  Scenario: Enabled sessions reuse sockets with full current DAG input
    Given an explicitly enabled Codex session and a local Responses socket
    When two compatible requests extend the current DAG
    Then one connection carries both complete inputs without a previous response identity

  Scenario: Changed history and reset preserve full current input
    Given a successful reusable Codex socket response
    When history changes and the session socket resets before another request
    Then the next request contains full input without a stale previous response identity

  Scenario: Unsupported handshakes fail without transport downgrade
    Given a Codex server rejecting WebSocket upgrade as unsupported
    When an enabled session samples a response
    Then the rejected upgrade remains visible and no SSE request or completion is published

  Scenario: Authentication fails while accepted disconnects recover on WebSocket
    Given a rejected credential or a socket interrupted after accepting a request
    When the transport reports the failure
    Then authentication remains terminal while a bounded full-input WebSocket replacement completes without SSE

  Scenario: Cancellation retires the socket within a bounded deadline
    Given an accepted Codex socket streaming partial output
    When its consumer cancels without server acknowledgement
    Then the socket is retired promptly and the next request starts a fresh full exchange

  Scenario: Socket and SSE transcripts preserve observable provider behavior
    Given equivalent Codex transcripts over a socket and SSE and an ordinary Responses client
    When each consumes text reasoning usage and tool output
    Then Codex produces equivalent observable events and ordinary Responses remains on HTTP
