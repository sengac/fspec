@done
@parity
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: EmbeddedFspecBackend smoke test
  End-to-end smoke test exercising EmbeddedFspecBackend against a real
  tempdir-backed WorkUnitsWatcher hosting a real SharedFspecService.
  Round-trips list_work_units().await and the work_units_rx()
  broadcast subscription after a fresh fs event.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want EmbeddedFspecBackend to round-trip list_work_units and work_units_rx against a real WorkUnitsWatcher
    So that the in-process RPC seam from RPC-005/006 is observably wired through the new trait boundary

  Scenario: EmbeddedFspecBackend smoke test round-trips list_work_units and work_units_rx
    Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService
    And an EmbeddedFspecBackend constructed via `EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service)`
    When the test calls `backend.list_work_units().await`
    Then the returned Vec<WorkUnitInfo> equals the watcher's snapshot()
    When the test subscribes via `backend.work_units_rx()` and the workspace receives a fresh fs event
    Then a Vec<WorkUnitInfo> reflecting the new state arrives on the receiver within 5 seconds
