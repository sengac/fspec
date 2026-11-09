@BUG-582
@done
@rpc
@security
@bug-fix
@rust
@BUG-619
Feature: wss connects choose the ring crypto provider
  """
  KGRAPH-072 (nanograph 1.3.0) links aws-lc-rs into the fspec binary through the AWS SDK, beside the ring provider RPC-456 uses. rustls 0.23 then cannot pick a process-level crypto provider, and tokio-tungstenite's rustls connector (ClientConfig::builder()) panics on every wss:// connect: fspec status, fspec client and the daemon switcher against a TLS daemon. fspec's main installs ring as the process default before anything else runs, and codelet_rpc_server::ws_connect installs it when a process (a library caller, a test) has none. Audit of the other rustls clients: reqwest 0.12 (providers, OAuth, web search, self-update, rmcp streamable HTTP) and ureq 2 build their configs with builder_with_provider(ring) and are not affected; every tokio-tungstenite connect (ws_connect, the TUI's connect_with_supervisor, the bridge relay) uses the process default.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The fspec process uses the ring provider: main installs it first; ws_connect installs it when none is installed
  #   2. A wss:// connect never panics for want of a crypto provider with ring and aws-lc-rs both linked
  #   3. reqwest and ureq pick ring explicitly; tokio-tungstenite uses the process default
  #
  # EXAMPLES:
  #   1. fspec status --connect wss://localhost:<port> against an untrusted certificate reports the TLS failure, no panic
  #   2. ws_connect in a binary linking both providers returns the certificate error instead of panicking
  #
  # ========================================
  Background: User Story
    As a user connecting fspec's own clients to a TLS daemon
    I want wss:// connects to work without the client panicking
    So that fspec status, fspec client and the daemon switcher reach TLS daemons whatever crypto backends other dependencies link in

  Scenario: fspec status reports a TLS failure instead of panicking
    Given a TLS daemon listener on localhost whose certificate the client does not trust
    When "fspec status --connect wss://localhost:<port>" runs
    Then it exits with code 1 and says the connection failed
    And its stderr names no CryptoProvider panic

  Scenario: ws_connect chooses ring when both crypto providers are linked
    Given a test binary that links both the ring and the aws-lc-rs rustls providers and installed no default
    When ws_connect dials wss://localhost:<port> of a TLS listener with an untrusted certificate
    Then it returns an error about the certificate
    And the process default crypto provider is ring
