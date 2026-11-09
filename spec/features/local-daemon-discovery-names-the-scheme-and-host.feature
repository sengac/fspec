@done
@rpc
@cli
@multi-workspace
@bug-fix
@rust
@BUG-595
Feature: Local daemon discovery names the scheme and host
  """
  Every discovery entry (daemons/<pid>.json and daemon.json, one schema written by common::write_daemon_json) appends "scheme" (ws or wss) and "host" (the host a local client dials). The daemon derives them from its --bind, whether it serves TLS and the new --tls-name (common::discovery_listener): the --bind host as given; an unspecified bind (0.0.0.0, ::) is 127.0.0.1 for a plain daemon and needs --tls-name for a TLS daemon, which then records that name. Combined mode writes ws and 127.0.0.1. Readers (DaemonEntry for listings, DaemonHandshake for daemon.json) take an absent scheme as ws and an absent host as 127.0.0.1, the values every entry written before the fields existed stood for; any other scheme, or an empty host, makes the entry unreadable. Every discovery-based connect builds <scheme>://<host>:<port> with the entry's url(): the daemon switcher's local daemons, fspec client (and its MCP front) and fspec status without --connect. Certificate trust is unchanged (private CAs are RPC-458). Product owner decision 2026-11-07 (option b).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every discovery entry records the scheme the daemon serves: ws for a plain listener, wss for a TLS listener; combined mode always writes ws
  #   2. An entry without the scheme field is ws, explicitly; any other value makes the entry unreadable
  #   3. Every discovery-based connect builds its URL from the entry
  #   4. The host field is the --bind host as given; an unspecified bind is 127.0.0.1 for a plain daemon; a TLS daemon on an unspecified bind needs --tls-name and records it
  #   5. --tls-name is accepted only with --tls-cert; an entry without host is 127.0.0.1, explicitly; an empty host is unreadable
  #   6. A discovery-based client dials <scheme>://<host>:<port>; certificate trust does not change
  #
  # EXAMPLES:
  #   1. --bind 0.0.0.0:3777 with TLS and --tls-name myhost.lan writes scheme wss, host myhost.lan; the switcher lists wss://myhost.lan:3777
  #   2. A plain loopback daemon writes scheme ws, host 127.0.0.1
  #   3. An entry written by fspec 0.11.0 (no scheme, no host) is listed as ws://127.0.0.1:<port>
  #   4. An entry with scheme https is unreadable
  #   5. --bind 0.0.0.0:3777 with TLS and no --tls-name refuses to start
  #   6. --bind 192.168.1.5:3777 with TLS records host 192.168.1.5; --bind [::1]:0 is dialed at ws://[::1]:<port>
  #
  # ========================================
  Background: User Story
    As a user running a local fspec daemon that serves TLS
    I want the daemon switcher, fspec client and fspec status without --connect, and fspec-paseo to dial the scheme and host the daemon serves
    So that discovery-based clients reach the daemon instead of failing at the TLS handshake

  Scenario: A discovery entry records the scheme and host the daemon serves
    Given a TLS daemon on port 3777 whose discovery host is myhost.lan
    When its discovery entry is written and the live daemons are listed
    Then the entry holds "scheme": "wss" and "host": "myhost.lan"
    And the listed daemon's URL is wss://myhost.lan:3777
    And a plain daemon's entry holds "scheme": "ws" and "host": "127.0.0.1" and is listed as ws://127.0.0.1:<port>

  Scenario: An entry written before the fields existed is a plain loopback daemon
    Given a daemons/<pid>.json and a daemon.json for a live process with only port, pid, workspace, started_at and version
    When the live daemons are listed and daemon.json is read
    Then both name ws://127.0.0.1:<port>

  Scenario: An entry with an unknown scheme or an empty host is unreadable
    Given discovery entries for live processes with "scheme": "https" and with "host": ""
    When the live daemons are listed and daemon.json is read
    Then the listing deletes both entries
    And reading daemon.json fails naming the invalid field

  Scenario: The discovery host follows the bind, TLS and --tls-name
    Given the binds 127.0.0.1:0, localhost:0, [::1]:0, 0.0.0.0:0, :::0 and 192.168.1.5:3777
    When each is turned into a discovery scheme and host, plain and with TLS
    Then a plain daemon is ws with the bind host, and 127.0.0.1 for 0.0.0.0 and ::
    And a TLS daemon is wss with the bind host, and with --tls-name myhost.lan its host is myhost.lan
    And a TLS daemon on 0.0.0.0 or :: without --tls-name is refused with a message naming --tls-name

  Scenario: A TLS daemon on an unspecified bind without --tls-name refuses to start
    Given a certificate, its key and a token
    When "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key" runs without --tls-name
    Then it exits with an error naming --tls-name

  Scenario: fspec status without --connect dials the TLS daemon's scheme and host
    Given "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key --tls-name localhost" running with a token
    When "fspec status" runs without --connect
    Then it dials wss://localhost:<port>

  Scenario: The daemon switcher lists a local TLS daemon with its scheme and host
    Given a live local daemon entry with "scheme": "wss" and "host": "localhost"
    When the switcher builds its local daemon choice
    Then the choice's URL is wss://localhost:<port>
