@done
@RPC-012
@rust
@tui
@infrastructure
@rpc
Feature: RPC-012 source-shape invariants — no Mutex/RwLock/atomics/transport imports in store/
  """
  RPC-012 — Source-shape regression that pins the store/ module's
  invariants: no Mutex/RwLock/atomic types, no runtime constructors, no
  direct transport-layer imports. Mirrors the RPC-008/RPC-009 source-shape
  pattern (tests/source_shape_*.rs) extended to the new state-ownership
  surface.
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want a source-shape regression that locks the new store/ module's no-Mutex/no-transport invariants
    So that future refactors cannot accidentally introduce hidden interior mutability or transport-layer coupling on the UI-side state

  Scenario: Source-shape regression forbids Mutex/RwLock/atomics in store/
    Given the directory rust/fspec-tui/src/store/
    When the test scans every .rs file under that directory
    Then no file contains "std::sync::Mutex"
    And no file contains "tokio::sync::Mutex"
    And no file contains "std::sync::RwLock"
    And no file contains "tokio::sync::RwLock"
    And no file contains "AtomicUsize" or "AtomicBool" in a struct field type
    And no file contains "tokio::runtime::Builder"
    And no file contains "tokio::runtime::Runtime::new"

  Scenario: Source-shape regression forbids transport-layer imports in store/
    Given the directory rust/fspec-tui/src/store/
    When the test scans every .rs file under that directory
    Then no file contains the import "codelet_napi::"
    And no file contains the import "codelet_core::"
    And no file contains the import "tarpc::"
    And no file contains the import "tokio_tungstenite::"

  Scenario: Every file under rust/fspec-tui/src/ is under 300 LoC
    Given the directory rust/fspec-tui/src/
    When the test counts the line-count of every .rs file under that directory
    Then store/board.rs has fewer than 300 lines
    And store/agent_view.rs has fewer than 300 lines
    And store/mod.rs has fewer than 300 lines
    And views/navigator.rs has fewer than 300 lines
    And views/board.rs has fewer than 300 lines
    And views/agent.rs has fewer than 300 lines
    And app.rs has fewer than 300 lines
