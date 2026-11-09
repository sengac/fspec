@done
@rpc
@security
@rust
@RPC-458
Feature: Clients trust a daemon's private CA
  """
  fspec's client (codelet_rpc_server::ws_connect, tokio-tungstenite with rustls) trusted only the public web PKI roots, so a daemon with a private-CA or self-signed certificate (RPC-456) could never be reached by fspec's own clients. A client now trusts the public roots plus the PEM certificates of FSPEC_DAEMON_CA (every connect) and of a saved daemon's "ca" file (fspec daemons add --ca <pem>, used when connecting to that saved daemon). The certificate is still verified against the dialed host; there is no skip-verification mode. A CA file that is missing, unreadable or holds no PEM certificate is an error naming the file and where it came from, before any connection is tried; fspec daemons add --ca checks the file the same way before saving and refuses --ca for a ws:// URL. Every connect uses the same trust: the TUI WebSocket backend (connect, MCP connect, reconnect supervisor), the daemon switcher, fspec client and fspec status. Product owner, 2026-11-07 (BUG-595 answer): private-CA trust is this card.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A client trusts the public roots plus FSPEC_DAEMON_CA and the saved daemon's ca; the certificate is still verified against the dialed host
  #   2. No skip-verification mode: a certificate that does not chain to a trusted root or does not name the host fails as a TLS verification error naming the host
  #   3. A missing, unreadable or certificate-less CA file is an error naming the file and its source; nothing falls back to the public roots alone
  #   4. Every client connect uses the same trust
  #   5. fspec daemons add --ca checks the file the same way before saving and refuses --ca for a ws:// URL; nothing is saved on a refusal
  #
  # EXAMPLES:
  #   1. FSPEC_DAEMON_CA=ca.pem: fspec status --connect wss://localhost:<port> reaches a daemon serving server.pem
  #   2. Without FSPEC_DAEMON_CA the same connect fails with a TLS verification error
  #   3. FSPEC_DAEMON_CA=/missing.pem fails before connecting, naming the file
  #   4. fspec daemons add lab wss://lab.lan:3777 --token t --ca lab-ca.pem saves the CA; switching to lab trusts it
  #   5. A certificate for other.lan dialed as localhost still fails with FSPEC_DAEMON_CA set
  #   6. fspec daemons add lab wss://localhost:3777 --token t --ca empty.pem (no certificate) is refused naming empty.pem; lab is not saved
  #
  # ========================================
  Background: User Story
    As a user connecting fspec's clients to a daemon whose TLS certificate comes from a private CA or is self-signed
    I want to tell fspec which CA certificate to trust for that daemon
    So that the switcher, fspec client and fspec status reach the daemon over verified TLS without a public certificate

  Scenario: FSPEC_DAEMON_CA makes a private-CA daemon reachable
    Given a TLS daemon serving a certificate for localhost signed by the test CA, with a token
    When "fspec status --connect wss://localhost:<port>" runs with FSPEC_DAEMON_CA naming the test CA and the token
    Then it reports the daemon alive

  Scenario: Without the CA the certificate is refused
    Given the same TLS daemon
    When "fspec status --connect wss://localhost:<port>" runs without FSPEC_DAEMON_CA
    Then it exits with code 1 and names a certificate verification failure

  Scenario: A CA file that cannot be used is an error before connecting
    Given FSPEC_DAEMON_CA names a missing file, and then a file that holds no PEM certificate
    When a client connects to wss://localhost:<port>
    Then each connect fails naming the file and FSPEC_DAEMON_CA

  Scenario: The trusted CA does not relax host verification
    Given a TLS daemon serving a certificate for localhost signed by the test CA
    When a client trusting the test CA dials it as wss://127.0.0.2:<port>
    Then the connect fails with a certificate verification error

  Scenario: A saved daemon carries its own CA
    Given "fspec daemons add lab wss://localhost:<port> --token t --ca ca.pem"
    When the saved daemon lab is connected to by name without FSPEC_DAEMON_CA
    Then daemons.json records the absolute path of ca.pem for lab
    And the connection is made trusting ca.pem

  Scenario: Saving a daemon with an unusable CA file is refused
    Given a file empty.pem that holds no PEM certificate
    When "fspec daemons add lab wss://localhost:3777 --token t --ca empty.pem" runs, and then the same with a ws:// URL and ca.pem
    Then each exits with an error naming the problem, empty.pem naming the file
    And lab is not saved
