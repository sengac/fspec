@BUG-582
@done
@TUI-123
@tui
@security
@multi-workspace
@rust
Feature: Add and remove remote daemons from the TUI as easily as workspaces
  """
  Daemon management is as discoverable as workspace management. The board
  header label "Daemon: <name> ▾" is clickable like the workspace label and
  opens the daemon switcher; `s` still opens it; the Settings menu gains
  "s Daemons" (the switcher) and "w Workspaces" (the workspace dropdown), and
  the board help lists both keys.

  codelet-common saved_daemons validates names (trimmed, no control
  characters, at most 64 characters) and URLs (ws:// or wss:// with a host;
  no credentials, query or fragment; ws:// only for loopback; non-loopback
  needs a token; a trailing slash is dropped, an explicit port kept) for both
  `fspec daemons add` and the TUI. Every write holds BUG-477's SpecLock on
  the ~/.fspec directory and replaces daemons.json through a uniquely named
  owner-only (0600) temp file; a malformed or unreadable file is an error
  naming its path and is never overwritten.

  codelet-fspec-tui: daemon_connect.rs connects within a timeout and names
  the failure (connection refused, timed out, TLS handshake failed, token
  rejected with 401). The App checks name clashes against every listed
  daemon (saved and local, ignoring case), shows a refusal under the field it
  concerns, connects after saving a new daemon (it stays saved if that
  fails), asks y/n before removing, and keeps the current connection when
  the connected daemon is edited (a notice says the change applies on the
  next connect) or removed (the header then shows its URL). The switcher
  shows a corrupt daemons.json as an error inside the panel, takes mouse
  clicks like the workspace dropdown and truncates with an ellipsis; the
  form scrolls long values and stays usable at the board's 64-column minimum.
  Tests: rust/common/tests/tui123_saved_daemon_rules.rs,
  rust/fspec/tests/tui123_concurrent_daemons_add.rs,
  rust/fspec-tui/tests/tui123_daemon_connect_failures.rs,
  rust/fspec-tui/tests/tui123_daemon_switcher_access.rs,
  rust/fspec-tui/tests/tui123_daemon_form_and_removal.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Discoverable like workspaces: clicking the board header label 'Daemon: <name> ▾' opens the daemon switcher (as clicking the workspace label opens the workspace dropdown); the 's' key keeps working; the menu bar gains a Daemons entry that opens the switcher; the Help dialog lists the key
  #   2. The open switcher lists live local daemons and saved daemons, marks the connected one, and shows the key hint 'Enter connect · a add · e edit · d remove · Esc close'; rows are clickable and a click outside closes it, like the workspace dropdown
  #   3. The add dialog has three fields: name, URL and token; Tab/Shift+Tab move between fields, Enter saves, Esc cancels; pasted text has newlines removed; the token field is always masked and never echoed, logged or shown in errors or Debug output
  #   4. Name validation: required, trimmed, no control characters, at most 64 characters, and unique against saved daemons and live local daemon names (case-insensitive); a clash names the existing entry
  #   5. URL validation: must parse as ws:// or wss:// with a host; http(s), missing scheme, empty host, credentials in the URL, a query or fragment are refused with the reason; non-loopback hosts must use wss:// and need a token; ws:// is allowed only for loopback (localhost, 127.0.0.0/8, ::1); a trailing slash is normalised away; an explicit port is kept
  #   6. A refused save reopens the dialog with everything typed (token kept but masked) and the reason shown on the field it concerns; nothing is written
  #   7. Saving writes ~/.fspec/daemons.json atomically with owner-only 0600 permissions under a lock, so a concurrent 'fspec daemons add' or another TUI cannot lose or corrupt entries; the switcher re-reads the file every time it opens
  #   8. An unreadable or corrupt daemons.json is an error shown in the switcher (never an empty list), and saving refuses rather than overwriting it
  #   9. Saving never requires the daemon to be reachable (daemons may be offline); connecting to an unreachable daemon or one that rejects the token shows the reason (refused, timeout, TLS failure, 401 token rejected) and keeps the current connection
  #   10. Edit (e) opens the dialog filled with name and URL and an empty token field; an empty token keeps the saved token; renaming replaces the old entry; editing the connected daemon changes what is used on the next connect, and the user is told so
  #   11. Delete (d or Delete) removes the highlighted saved daemon and its stored token; local (discovered) daemons cannot be edited or removed and the reason is shown; deleting never stops or detaches sessions on that daemon
  #   12. Deleting the daemon the TUI is connected to keeps the current connection; the header then names it by URL, and switching away leaves it unreachable from the switcher until it is added again
  #   13. The switcher and dialog fit narrow terminals: labels truncate with an ellipsis, the dialog keeps its fields usable down to the board's minimum width, and long URLs scroll inside the field
  #   14. Delete asks first: 'Remove saved daemon <name> and its stored token? y/n'; only y removes it, any other key cancels (product owner, 2026-11-05)
  #   15. Saving a new daemon saves it and then switches to it immediately; if connecting fails (unreachable, TLS, 401) the daemon stays saved, the TUI stays connected to the current daemon and the reason is shown (product owner, 2026-11-05)
  #   16. Daemons are reachable from the header dropdown (the clickable 'Daemon: <name> ▾' label, next to the workspace label) and from a Settings menu entry 'Daemons'; workspaces likewise get a Settings menu entry 'Workspaces' that opens the workspace dropdown with its add/remove keys (product owner, 2026-11-05)
  #
  # EXAMPLES:
  #   1. On the board connected to 'alpha', a left click on the header label 'Daemon: alpha ▾' opens the daemon switcher; pressing 's' opens it too
  #   2. Opening the Settings menu shows 'M Mux', 'P Providers', 's Daemons' and 'w Workspaces'; executing 's Daemons' opens the daemon switcher and 'w Workspaces' opens the workspace dropdown
  #   3. The board help ('?') lists 's  Daemons (switch, add, edit, remove)' and 'w  Workspaces (switch, add, remove)'
  #   4. With local daemon 'alpha' (connected) and saved daemon 'work', the open switcher shows 'alpha (connected)' and 'work (saved)' and the hint 'Enter connect · a add · e edit · d remove · Esc close'
  #   5. With the switcher open, clicking the 'work' row switches to work; clicking anywhere outside the panel closes the switcher
  #   6. In the add dialog the user types name 'work', Tab, URL 'wss://fspec.example.com', Tab, token 'secret-token'; the token row shows 12 dots and no frame, Debug output of the submitted action or error ever contains 'secret-token'
  #   7. Pasting 'wss://fspec.example.com\n' into the URL field enters 'wss://fspec.example.com' (the newline is removed)
  #   8. Names '  work  ' saves as 'work'; '' is refused as required; 'wo\trk' (pasted tab) is refused for a control character; a 65-character name is refused as longer than 64
  #   9. With saved daemon 'work' and local daemon 'alpha', adding 'WORK' is refused with 'a daemon named work is already saved' and adding 'Alpha' with 'alpha is the name of a local daemon'
  #   10. URL forms: 'http://host' and 'https://host' refused (ws:// or wss:// only); 'fspec.example.com:7443' refused (missing scheme); 'wss://' and 'wss://:7443' refused (no host); 'wss://bob:pw@host' refused for credentials and the reason does not repeat the URL; 'wss://host/?x=1' and 'wss://host/#a' refused (query or fragment)
  #   11. 'ws://10.0.0.5:9000' refused (non-loopback needs wss://); 'wss://fspec.example.com' with no token refused (needs a token); 'ws://localhost:4000', 'ws://127.0.0.2:4000' and 'ws://[::1]:4000' accepted without a token; 'wss://fspec.example.com:8443/' saves as 'wss://fspec.example.com:8443' (slash removed, port kept)
  #   12. Saving 'wss://fspec.example.com' without a token reopens the dialog with name, URL and the masked token kept, the reason shown under the Token field, and daemons.json unchanged
  #   13. Sixteen concurrent 'fspec daemons add d<i> ws://127.0.0.1:<port>' processes against one HOME leave all sixteen entries in daemons.json, which stays mode 0600 with no temp files left behind
  #   14. A daemon added by 'fspec daemons add' while the TUI runs appears the next time the switcher opens
  #   15. With daemons.json containing '{not json', opening the switcher shows 'could not be read' with the file path inside the switcher (no daemon rows), and saving a new daemon is refused; the file still contains '{not json'
  #   16. Saving 'wss://offline.example.com' with a token succeeds while that host is down; connecting to it shows the reason and the header still names the current daemon
  #   17. Connect failures by kind: nothing listening -> 'connection refused'; a host that accepts but never answers -> 'timed out'; a plain TCP server behind wss:// -> 'TLS handshake failed'; a daemon answering 401 -> 'rejected the token (401)'
  #   18. Editing saved daemon 'work' opens the dialog with name 'work', URL 'wss://fspec.example.com' and an empty token row reading '(empty keeps the saved token)'; renaming it to 'work2' leaves only 'work2' with the old token
  #   19. Editing 'work' while connected to it saves the change and tells the user it applies on the next connect; the connection is kept and the header follows a rename
  #   20. 'd' on saved daemon 'work' removes 'work' and its token from daemons.json after confirming; 'd' or 'e' on local daemon 'alpha' shows 'alpha is a local daemon ... cannot be edited or removed'; sessions on the daemon are neither stopped nor detached
  #   21. Connected to saved daemon 'work' at 'wss://fspec.example.com', deleting 'work' keeps the connection and the header reads 'Daemon: wss://fspec.example.com ▾'; the switcher no longer lists 'work'
  #   22. At 64 columns (the board minimum) with a 90-character URL, the dialog fits the width, its three fields remain visible and the URL field shows the end of the URL behind an ellipsis; a long daemon name in the switcher and header is cut with '…'
  #   23. 'd' on 'work' shows 'Remove saved daemon work and its stored token? y/n'; 'n' or Esc keeps 'work'; 'y' removes it
  #   24. Adding 'work' at a reachable daemon saves it and switches the board to 'work'; adding 'offline' at an unreachable daemon saves it, keeps the board on 'alpha' and shows 'Saved daemon offline, but could not connect: connection refused'
  #
  # ========================================
  Background: User Story
    As a fspec user working with daemons on other machines
    I want to add, edit, connect to and remove remote daemons from the TUI the same way I add and remove workspaces
    So that I can manage remote daemons without leaving the TUI or knowing hidden keys

  Scenario: Clicking the header daemon label opens the switcher
    Given the board is connected to local daemon "alpha"
    When the user clicks the header label "Daemon: alpha ▾"
    Then the daemon switcher opens
    And pressing "s" on the board opens it as well

  Scenario: The Settings menu opens the daemon switcher and the workspace dropdown
    Given the board with its menu bar
    When the user opens the Settings menu
    Then it lists "M Mux", "P Providers", "s Daemons" and "w Workspaces"
    And executing "s Daemons" opens the daemon switcher
    And executing "w Workspaces" opens the workspace dropdown

  Scenario: The board help lists the daemon and workspace keys
    Given the board
    When the user opens the board help with "?"
    Then it lists "s" for daemons and "w" for workspaces

  Scenario: The open switcher marks the connected daemon and shows its keys
    Given local daemon "alpha" is connected and daemon "work" is saved
    When the daemon switcher is open
    Then it shows "alpha (connected)" and "work (saved)"
    And its border reads "Enter connect · a add · e edit · d remove · Esc close"

  Scenario: Switcher rows are clickable and a click outside closes it
    Given the daemon switcher is open listing "alpha" and "work"
    When the user clicks the "work" row
    Then the board switches to "work"
    And a click outside the open switcher closes it

  Scenario: The switcher re-reads daemons.json every time it opens
    Given the TUI is running with no saved daemons
    When "work" is saved by another process and the switcher is opened
    Then the switcher lists "work"

  Scenario: A corrupt daemons.json is an error in the switcher and is never overwritten
    Given daemons.json contains "{not json"
    When the daemon switcher is opened
    Then the switcher shows that daemons.json could not be read, with its path, and no daemon rows
    And saving a new daemon is refused
    And daemons.json still contains "{not json"

  Scenario: The token is masked everywhere
    Given the add daemon dialog is open
    When the user enters name "work", URL "wss://fspec.example.com" and token "secret-token"
    Then the token row shows 12 dots
    And "secret-token" appears in no frame and in no Debug output of the saved form

  Scenario: Pasted newlines are removed
    Given the add daemon dialog is open on the URL field
    When the user pastes "wss://fspec.example.com" followed by a newline
    Then the URL field holds "wss://fspec.example.com"

  Scenario: Daemon names are validated
    Given an empty fspec home
    When these names are saved for "ws://127.0.0.1:4000":
      | name          | outcome                            |
      | "  work  "    | saved as "work"                    |
      | ""            | refused: a name is required        |
      | "wo" TAB "rk" | refused: control character         |
      | 65 letters    | refused: longer than 64 characters |
    Then each name has the outcome in the table

  Scenario: Daemon names are unique against saved and local daemons, ignoring case
    Given saved daemon "work" and local daemon "alpha"
    When the user adds "WORK" and then "Alpha"
    Then "WORK" is refused naming the saved daemon "work"
    And "Alpha" is refused naming the local daemon "alpha"
    And nothing new is saved

  Scenario: Daemon URLs are validated and normalised
    Given an empty fspec home
    When these URLs are saved:
      | url                           | token | outcome                                |
      | http://fspec.example.com      | t     | refused: ws:// or wss:// only          |
      | https://fspec.example.com     | t     | refused: ws:// or wss:// only          |
      | fspec.example.com:7443        | t     | refused: missing scheme                |
      | wss://                        | t     | refused: no host                       |
      | wss://:7443                   | t     | refused: no host                       |
      | wss://bob:hunter2@example.com | t     | refused: credentials, URL not repeated |
      | wss://fspec.example.com/?x=1  | t     | refused: query or fragment             |
      | wss://fspec.example.com/#top  | t     | refused: query or fragment             |
      | ws://10.0.0.5:9000            | t     | refused: non-loopback needs wss://     |
      | wss://fspec.example.com       |       | refused: needs a token                 |
      | ws://localhost:4000           |       | saved                                  |
      | ws://127.0.0.2:4000           |       | saved                                  |
      | ws://[::1]:4000               |       | saved                                  |
      | wss://fspec.example.com:8443/ | t     | saved as wss://fspec.example.com:8443  |
    Then each URL has the outcome in the table

  Scenario: A refused save reopens the dialog with everything typed
    Given the add daemon dialog is open
    When the user saves "lan" at "ws://10.0.0.5:9000" with token "secret-token"
    Then the dialog reopens with name "lan", URL "ws://10.0.0.5:9000" and 12 token dots
    And the reason is shown under the URL field
    And daemons.json is unchanged

  Scenario: Concurrent fspec daemons add processes lose nothing
    Given an empty fspec home
    When 16 "fspec daemons add" processes run at the same time with different names
    Then daemons.json lists all 16 daemons
    And daemons.json is mode 0600 and no temporary file is left

  Scenario: Concurrent saves in one process lose nothing
    Given an empty fspec home
    When 16 threads save different daemons at the same time
    Then daemons.json lists all 16 daemons

  Scenario: An unreadable daemons.json is an error and is left in place
    Given daemons.json cannot be read as a file
    When the saved daemons are listed and a daemon is saved
    Then both fail naming the daemons.json path
    And daemons.json is left as it was

  Scenario: Connect failures are named by kind
    Given daemons that refuse the connection, never answer, do not speak TLS and reject the token with 401
    When the client connects to each of them
    Then the failures read "connection refused", "timed out", "TLS handshake failed" and "rejected the token (401 Unauthorized)"

  Scenario: Switching to a daemon that rejects the token keeps the current connection
    Given the board is connected to "alpha" and saved daemon "work" rejects the token
    When the user switches to "work"
    Then an error says daemon "work" rejected the token
    And the header still reads "Daemon: alpha ▾"

  Scenario: Saving a new daemon connects to it
    Given the board is connected to "alpha" and daemon "work" is reachable
    When the user adds daemon "work"
    Then "work" is saved
    And the header reads "Daemon: work ▾"

  Scenario: A new daemon that cannot be reached stays saved
    Given the board is connected to "alpha" and daemon "offline" refuses connections
    When the user adds daemon "offline"
    Then "offline" is saved
    And the header still reads "Daemon: alpha ▾"
    And an error says "Saved daemon offline, but could not connect" and "connection refused"

  Scenario: Renaming a saved daemon replaces it and keeps its token
    Given saved daemon "work" at "wss://fspec.example.com" with token "secret-token"
    When the user edits "work" and renames it to "work2" leaving the token empty
    Then the edit dialog showed URL "wss://fspec.example.com" and an empty token row
    And only "work2" is saved, with token "secret-token"

  Scenario: Editing the connected daemon applies on the next connect
    Given the board is connected to saved daemon "work"
    When the user edits "work" to "wss://fspec2.example.com" and renames it to "work2"
    Then the board keeps its connection
    And the header reads "Daemon: work2 ▾"
    And a notice says the change applies the next time the TUI connects to it

  Scenario: Removing a saved daemon asks first
    Given the switcher highlights saved daemon "work"
    When the user presses "d"
    Then the prompt reads "Remove saved daemon work and its stored token? y/n"
    And answering "n" keeps "work"
    And pressing "d" and answering "y" removes "work" and its token

  Scenario: Local daemons cannot be edited or removed
    Given the switcher highlights local daemon "alpha"
    When the user presses "e" and then "d"
    Then both are refused because "alpha" is a local daemon
    And no removal prompt is shown

  Scenario: Removing the connected daemon keeps the connection and its sessions
    Given the board is connected to saved daemon "work" at "wss://fspec.example.com" with an open session
    When the user removes "work" and confirms
    Then the header reads "Daemon: wss://fspec.example.com ▾"
    And no session was destroyed or interrupted and the session tab is still open
    And the switcher no longer lists "work"

  Scenario: The switcher and the dialog fit a 64-column terminal
    Given a 64-column terminal and a daemon with a 64-character name
    When the switcher is open and then the add dialog holds a 90-character URL
    Then the name is cut with "…" in the header and in the switcher
    And the dialog fits within 64 columns with all three fields visible
    And the URL field shows the end of the URL after "…"
