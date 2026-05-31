@done
@tui
@rust
@infrastructure
@parity
@rpc
@tarpc
@websocket
@RPC-009
@critical
Feature: Cross-transport App-layer parity (RPC-009)
  """
  Two integration tests under codelet/fspec-tui/tests/: (a) embedded_app_smoke.rs constructs an EmbeddedFspecBackend wrapping a real tempdir-backed WorkUnitsWatcher (codelet_core::work_units) hosting a real SharedFspecService (codelet_rpc), wraps it in App::new, drives one render cycle, mutates `<tempdir>/spec/work-units.json` to add a third entry, awaits the broadcast via the work_units subscriber task (bounded `tokio::time::timeout(Duration::from_millis(200), ...)` — NO sleep), drives another render cycle, and asserts the rendered left pane reflects the new state; (b) ws_app_smoke.rs spawns a real `bind_and_serve` rpc-server on 127.0.0.1:0 against the same fixture, builds a `WebSocketFspecBackend::connect(ws_url)`, wraps it in App, and asserts the same observable left-pane behaviour. Both tests reuse the Q-FIX-1 fixtures from codelet/fspec-tui/tests/common/mod.rs (temp_service, start_ws_server, test_app, render_one_frame, buffer_to_rows). The cross-transport assertion compares the post-mutation row text from each App's rendered buffer — they must be byte-identical (or at minimum semantically identical: same set of work-unit ids/statuses in the same order).
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want a Rust integration test that drives the same scripted scenario (mutate work-units.json on disk → render frame) against an App built on EmbeddedFspecBackend AND another App built on WebSocketFspecBackend (real bind_and_serve rpc-server on 127.0.0.1:0)
    So that the SAME compiled crate produces an identical user-observable left pane on either transport — proving the FspecBackend trait seam at the App layer, not just at the trait surface

  Scenario: Embedded App smoke — mutate spec/work-units.json on disk and observe the rendered left pane reflects the new state
    Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService seeded with [AUTH-001 done, AUTH-002 implementing]
    And an `EmbeddedFspecBackend` constructed via `EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service)`
    And an `App::new(Arc::new(backend))` rendered onto an 80x24 TestBackend
    When the App's bootstrap completes and one frame is rendered
    Then the rendered buffer's left pane contains "AUTH-001 done" and "AUTH-002 implementing"
    When the test rewrites `<tempdir>/spec/work-units.json` to add a third entry AUTH-003 backlog
    And the work_units subscriber task observes the broadcast within 200ms
    And the App processes `Action::WorkUnitsLoaded` and renders another frame
    Then the rendered buffer's left pane contains "AUTH-003 backlog"

  Scenario: WS App smoke — spawn rpc-server and observe the rendered left pane reflects the new state
    Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService seeded with [AUTH-001 done, AUTH-002 implementing]
    And a `bind_and_serve` rpc-server bound to 127.0.0.1:0 against that service
    And a `WebSocketFspecBackend::connect(ws_url).await` against the resulting ws://127.0.0.1:<port>/ url
    And an `App::new(Arc::new(backend))` rendered onto an 80x24 TestBackend
    When the App's bootstrap completes and one frame is rendered
    Then the rendered buffer's left pane contains "AUTH-001 done" and "AUTH-002 implementing"
    When the test rewrites `<tempdir>/spec/work-units.json` to add a third entry AUTH-003 backlog
    And the work_units subscriber task observes the broadcast within 200ms
    And the App processes `Action::WorkUnitsLoaded` and renders another frame
    Then the rendered buffer's left pane contains "AUTH-003 backlog"

  Scenario: Cross-transport parity — both Apps' rendered left panes are semantically identical post-mutation
    Given a shared tempdir-backed WorkUnitsWatcher fixture seeded with [AUTH-001 done, AUTH-002 implementing]
    And an App-on-EmbeddedFspecBackend rendered onto an 80x24 TestBackend
    And an App-on-WebSocketFspecBackend (against `bind_and_serve` on 127.0.0.1:0) rendered onto an 80x24 TestBackend
    When the test mutates the workspace's spec/work-units.json to add AUTH-003 backlog
    And both Apps process `Action::WorkUnitsLoaded` and render another frame
    Then the row sequence in each App's left-pane buffer band (rows containing "AUTH-") matches the same set of work-unit ids in the same order
    And no transport-specific divergence in id, status, or item formatting appears
