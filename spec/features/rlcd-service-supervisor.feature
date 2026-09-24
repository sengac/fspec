@done
@RLCD-001
@rlcd-001
@tools
@rlcd
Feature: RLCD service supervisor: config, health polling, local auto-spawn with port scanning

  """
  New module rust/tools/src/rlcd/ (config.rs, service.rs, error.rs; client.rs arrives in RLCD-002). Status: RLCD_STATUS: RwLock<RlcdStatus{reachable, base_url, model, last_check, spawn_note}>. Polling task: spawn-once guard, tokio::time::interval(10s), Handle::current() only. Config: load_rlcd_config()/load_rlcd_config_from(path) + key-preserving writer mirroring save_persisted_model_string_to. Files stay <300 lines.
  """

  Background: User Story
    As a developer or agent running fspec
    I want to have a healthy RLCD decision server reachable at a configured URL (or auto-spawned locally on a free port) without manual setup
    So that all downstream RLCD consumers (Decision tool, workflow gate, security layer) can call a live engine with zero friction

  # ========================================
  # CONFIG (rules 1, 2, example 1, 9)
  # ========================================

  Scenario: Fresh machine with no config file resolves the full defaults
    Given a user config directory with no fspec-config.json file
    When the RLCD config is loaded
    Then the resolved config uses url "http://127.0.0.1:8000"
    And the resolved config uses spawn port 8000
    And the resolved config uses spawn binary "rlcd"
    And the resolved config uses the default backend "rlcd"
    And the resolved config has enabled true
    And no config file has been written to disk

  Scenario: A malformed config file resolves to defaults without failing
    Given a user config directory whose fspec-config.json contains invalid JSON
    When the RLCD config is loaded
    Then the resolved config uses the default url "http://127.0.0.1:8000"
    And loading does not return an error

  Scenario: A partial config fills only the missing fields from defaults
    Given a user config directory whose fspec-config.json sets rlcd.url to "http://10.0.0.5:9000" only
    When the RLCD config is loaded
    Then the resolved config uses url "http://10.0.0.5:9000"
    And the resolved config uses the default spawn port 8000
    And the resolved config uses the default spawn binary "rlcd"

  Scenario: Writing the RLCD config preserves every unrelated key verbatim
    Given a user config directory whose fspec-config.json already contains "theme":"dark" and a providers section
    When the RLCD section is written with url "http://127.0.0.1:9100"
    Then the config file contains rlcd.url "http://127.0.0.1:9100"
    And the config file still contains "theme":"dark"
    And the config file still contains the providers section unchanged

  # ========================================
  # HEALTH POLLING + STATUS (rules 3, 4; examples 2, 3, 10)
  # ========================================

  Scenario: A healthy local server is detected and its reported model name is used
    Given a mock RLCD server on 127.0.0.1 whose /health returns {"status":"ready","model":"typed-decisions"}
    When the supervisor polls /health
    Then the RLCD status reports reachable true
    And the RLCD status carries the model name "typed-decisions"
    And no local server has been spawned

  Scenario: A down server flips the status to unreachable and back
    Given a mock RLCD server on 127.0.0.1 that answers /health then stops
    When the supervisor polls /health after the server stops
    Then the RLCD status reports reachable false
    And a single down transition warning has been logged

  Scenario: The model name from /health is never hardcoded
    Given a mock RLCD server whose /health reports model "custom-variant"
    When the supervisor polls /health and then a classification request is built
    Then the request carries model "custom-variant"

  Scenario: A remote URL is connect-only and never spawned
    Given a configured url pointing at remote host 192.168.1.50 port 9000
    And no server is running on that remote host
    When the supervisor checks RLCD availability
    Then the status reports unreachable
    And no local server spawn has been attempted
    And no pidfile has been created

  # ========================================
  # AUTO-SPAWN: PORT SCAN (rule 6, 7; examples 4, 5, 8)
  # ========================================

  Scenario: When the configured port is free it is used for the spawn
    Given the configured spawn port 8000 is free
    When the supervisor selects a port for spawning
    Then port 8000 is selected

  Scenario: A port occupied by a non-RLCD service is skipped in favor of the next free port
    Given port 8000 is occupied by a service that does not answer /health with the RLCD ready shape
    And port 8001 is free
    When the supervisor selects a port for spawning
    Then port 8001 is selected

  Scenario: A port occupied by a healthy RLCD server is reused without spawning
    Given port 8000 is occupied by a server whose /health returns {"status":"ready","model":"m1"}
    When the supervisor selects a port for spawning
    Then the supervisor connects to the existing server on port 8000
    And no new server process has been spawned

  Scenario: When every scanned port is occupied the spawn fails and fails open
    Given all 10 scanned ports are occupied by non-RLCD services
    When the supervisor attempts to ensure a local server
    Then the attempt reports unreachable
    And a single spawn-failure warning has been logged
    And consumers degrade without blocking

  # ========================================
  # AUTO-SPAWN: PIDFILE + DETACH (rule 6, 7; examples 6, 7)
  # ========================================

  Scenario: A stale pidfile is removed before spawning
    Given a pidfile ~/.fspec/rlcd/rlcd.pid whose pid no longer exists
    When the supervisor prepares to spawn a local server
    Then the stale pidfile has been removed
    And the spawn proceeds

  Scenario: A live pid that is still loading suppresses a duplicate spawn
    Given a pidfile pointing at a live process whose /health is not ready yet
    When the supervisor ensures a local server
    Then the supervisor waits within the poll budget
    And no second server process has been spawned

  Scenario: A local spawn detaches the server and records its pid
    Given a free port and a resolvable "rlcd" binary
    When the supervisor spawns the local server
    Then the process is started with arguments "serve --host 127.0.0.1 --port <chosen>"
    And its stdout and stderr are appended to ~/.fspec/logs/rlcd-serve.log
    And its pid is written to the pidfile
    And the process survives the fspec process (process group detached)

  # ========================================
  # AUTO-SPAWN: URL RE-ANCHORING (regression: live server on a port
  # DIFFERENT from the configured url — the 2026-09-24 incident where a
  # spawn/reuse on a different port left every consumer probing the
  # configured url)
  # ========================================

  Scenario: Reusing an existing healthy server on a different port re-anchors and persists the url
    Given a configured url that is down
    And a healthy RLCD server occupying a different port in the scan window
    When the supervisor ensures a local server
    Then the user config file now contains rlcd.url pointing at the occupied port
    And the supervisor's effective base url points at the occupied port
    And a fresh supervisor built from the persisted config reaches the server
    And no second server process has been spawned

  Scenario: A live pidfile pointing at a healthy server on a different port re-anchors the supervisor
    Given a pidfile pointing at a live process that serves /health on a port other than the configured url
    When the supervisor ensures a local server
    Then the supervisor connects to the healthy port found on the local loopback listeners
    And the user config file now contains rlcd.url pointing at that port
    And the pidfile is unchanged
    And no second server process has been spawned

  Scenario: A failed local spawn leaves the configured url untouched
    Given a configured url that is down and a spawn flow that fails (all ports occupied)
    When the supervisor ensures a local server
    Then the attempt reports unreachable
    And the user config file's rlcd.url is unchanged

  # ========================================
  # INTEGRATION (example 10)
  # ========================================

  Scenario: All consumers read the shared status and fail open together
    Given the RLCD status reports unreachable
    When the Decision tool, the workflow gate, and the security layer each act
    Then the Decision tool returns a structured error
    And the workflow gate executes the command with a warning
    And the security layer skips the RLCD stage while regex rules still apply
    And none of the three hard-blocks the caller
