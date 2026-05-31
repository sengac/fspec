@done
@parity
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: codelet/fspec-tui Cargo.toml + workspace shape
  Source-shape regressions for the codelet/fspec-tui crate's workspace
  registration, [dependencies] / [dev-dependencies] policy, and the
  ban on own-runtime construction. Mirrors the pattern in
  codelet/rpc-embedded/tests/architecture_invariants.rs and
  rpc_006_source_shape.rs — these tests do not exercise any runtime
  code path.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want codelet/fspec-tui's Cargo.toml + source tree to be tightly bounded (only the RPC-seam crates in [dependencies], codelet-core allowed only in [dev-dependencies], and zero own-runtime construction)
    So that the shipped TUI binary path stays clean of NAPI/core surface and the host-supplied tokio Handle invariant from RPC-005 Q9 is preserved at this new layer

  Scenario: codelet/fspec-tui is a fifth RPC-family workspace member with no binary entry point
    Given the codelet workspace currently lists rpc, rpc-types, rpc-embedded, and rpc-server as RPC-family members
    When I add codelet/fspec-tui to codelet/Cargo.toml's [workspace] members and create codelet/fspec-tui/Cargo.toml plus codelet/fspec-tui/src/lib.rs
    Then "codelet-fspec-tui" appears in the workspace member list
    And codelet/fspec-tui/Cargo.toml contains a [lib] section but NO [[bin]] section
    And running "cargo build -p codelet-fspec-tui" from codelet/ exits with status code 0

  Scenario: codelet/fspec-tui production dependencies include only RPC seam crates plus ratatui dependencies
    Given codelet/fspec-tui/Cargo.toml exists
    When I read its [dependencies] table
    Then the [dependencies] table contains exactly these workspace dependencies: codelet-rpc, codelet-rpc-types, codelet-rpc-embedded, codelet-rpc-server, ratatui, crossterm, tokio, async-trait, futures, tarpc, tokio-tungstenite, url, anyhow, tracing
    And the [dependencies] table contains "tui-popup" pinned to "0.6"
    And the [dependencies] table does NOT list codelet-napi
    And the [dependencies] table does NOT list codelet-core

  Scenario: codelet/fspec-tui dev-dependencies allow codelet-core for fixtures but never codelet-napi
    Given codelet/fspec-tui/Cargo.toml exists
    When I read its [dev-dependencies] table
    Then the [dev-dependencies] table contains insta with the yaml feature
    And the [dev-dependencies] table contains tempfile and tokio-test
    And the [dev-dependencies] table MAY list codelet-core (for real-service integration fixtures)
    And the [dev-dependencies] table does NOT list codelet-napi

  Scenario: codelet/fspec-tui contains no own-runtime construction calls
    Given a source-shape integration test that scans codelet/fspec-tui/src/
    When the test reads each .rs file and strips comments
    Then no file contains "tokio::runtime::Builder"
    And no file contains "runtime::Builder::new_multi_thread"
    And no file contains "runtime::Builder::new_current_thread"
    And no file contains "tokio::runtime::Runtime::new"
    And no file contains "Runtime::new()"

  Scenario: The existing scenario_7_* test in rpc-embedded is widened to scan fspec-tui
    Given the existing test codelet/rpc-embedded/tests/architecture_invariants.rs::scenario_7_embedded_transport_requires_tokio_handle_at_construction
    When I inspect its directory list
    Then it scans both "rpc-embedded/src" AND "fspec-tui/src" for forbidden runtime construction calls
    And the assertion message identifies which crate violated the invariant on failure
