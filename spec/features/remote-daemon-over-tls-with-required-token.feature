@done
@RPC-456
@rpc
@security
@rust
Feature: Daemon accepts remote clients over TLS with a required token
  """
  fspec daemon refused every non-loopback --bind, so a saved wss:// daemon
  worked only behind an external TLS proxy or tunnel. The daemon now
  accepts a non-loopback bind only when it serves TLS (--tls-cert and
  --tls-key, PEM) AND requires a bearer token (FSPEC_DAEMON_TOKEN, the
  RPC-447 check). codelet_rpc_server::tls::check_bind_security decides,
  load_tls_acceptor builds a rustls (ring provider) acceptor and
  bind_and_serve_tls wraps each connection in TLS before the WebSocket
  handshake. Loopback binds are unchanged (TLS optional there).
  Tests: rust/fspec/tests/rpc456_remote_daemon_over_tls.rs.
  """

  Background: User Story
    As a user running the fspec daemon on another machine
    I want to bind the daemon to a network interface behind TLS and a required token
    So that saved wss:// daemons work without an external proxy or tunnel and nobody else can reach my sessions

  Scenario: A network bind without TLS or a token is refused
    Given a daemon asked to bind "0.0.0.0:0"
    When it has no TLS certificate, or no token
    Then it refuses to start and names what is missing

  Scenario: A loopback bind needs neither TLS nor a token
    Given a daemon asked to bind "127.0.0.1:0" without TLS or a token
    Then the bind is allowed

  Scenario: A TLS daemon serves a wss client that presents the token
    Given a daemon serving TLS with a token
    When a wss client that trusts the certificate connects with the token
    Then the client's health request is answered

  Scenario: A TLS daemon rejects a client without the token
    Given a daemon serving TLS with a token
    When a wss client connects without the token
    Then the connection is rejected as unauthorized

  Scenario: The daemon command refuses a network bind without TLS
    Given the token is set
    When "fspec daemon --bind 0.0.0.0:0" runs without --tls-cert and --tls-key
    Then it exits with an error naming the missing TLS certificate

  Scenario: The daemon command serves a network bind with TLS and a token
    Given a certificate, its key and a token
    When "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key --tls-name localhost" runs
    Then a wss client presenting the token is answered
