@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@RPC-006
Feature: RPC-006 source-shape architecture invariants
  """
  Architecture

  Source-shape invariants the RPC-006 watcher lift MUST preserve or
  introduce. These tests are pure source-file inspection — no runtime
  spawn — so they fail loudly during cargo build whenever a future
  refactor accidentally crosses a boundary.

  - The fixture from RPC-005 must become unreachable from production code.
  - The embedded transport must continue to honour the runtime-Handle
  invariant from RPC-002 Q9 even though it now spawns a fan-out task.
  - The shared service crate (rust/rpc) widens its dep arrow: it MAY
  now depend on codelet-core (for the watcher lift) but MUST NOT
  depend on codelet-napi.
  - The rpc-server binary must still bind only to 127.0.0.1 (RPC-005
  rule [13]).
  - WorkUnitInfo must remain defined exactly once in rust/rpc-types,
  re-exported by both rust/napi AND rust/core wherever they
  reference it.
  - The embedded read path must remain bincode-free (zero-cost embedded
  push per RPC-002 §5.1).

  References: spec/attachments/RPC-006/plan.md ("Architecture conformance with RPC-002");
  RPC-005 architecture rules [1], [4], [10], [13];
  spec/attachments/RPC-006/ast-research-watcher-lift.md §3.
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want automated source-shape regressions that fail loudly when the runtime-Handle, dependency-arrow, single-source-of-truth, or loopback-bind invariants are violated
    So that future refactors cannot silently fork the type system, spawn rogue runtimes, or expose the daemon beyond loopback

  Scenario: default_fixture is unreachable from production code
    Given the rust/rpc crate after the watcher lift
    When I search the crate sources for the previous `default_fixture` symbol and for any production reference to `test_fixture`
    Then the symbol exists only inside a `#[cfg(test)]`-gated module or function and no production module path references it

  Scenario: Embedded transport reuses the host tokio runtime Handle for any fan-out it spawns
    Given the rust/rpc-embedded crate after RPC-006
    When I search its source for tokio runtime construction calls and for the spawn target of any internal fan-out task
    Then the crate contains no calls to tokio::runtime::Builder or tokio::runtime::Runtime::new and any background task is spawned via the stored host runtime Handle

  Scenario: Shared service crate codelet-rpc may depend on codelet-core but not on codelet-napi
    Given rust/rpc/Cargo.toml after the watcher lift
    When I inspect its [dependencies] table and the rust/rpc/src/ source for use statements naming codelet_napi or codelet_core
    Then the manifest declares a codelet-core dependency, declares no codelet-napi dependency, and the source contains no `use codelet_napi` import

  Scenario: rpc-server binary still binds 127.0.0.1 loopback only after the watcher integration
    Given the rpc-server binary main.rs after RPC-006 has been wired to construct a real WorkUnitsWatcher
    When I inspect rust/rpc-server/src/main.rs for the bind address literal
    Then the only bind address literal is 127.0.0.1:0 and no 0.0.0.0 or other non-loopback bind address appears in the binary source

  Scenario: WorkUnitInfo continues to be defined exactly once in rpc-types
    Given rust/rpc-types defines a public WorkUnitInfo struct with fields id, title, work_type, status, description, estimate, and epic
    When I inspect rust/napi and rust/core for any local definition of a struct named WorkUnitInfo and run cargo check on the workspace
    Then no other crate defines WorkUnitInfo locally, both crates re-export the type from rust/rpc-types where they reference it, and the workspace builds successfully

  Scenario: Embedded push path contains no bincode encode call
    Given the rust/rpc-embedded crate after RPC-006 exposes work_units_rx via a broadcast::Receiver
    When I search its source for bincode serialize or deserialize calls
    Then the crate contains no bincode::serialize or bincode::deserialize call sites because the embedded push path returns the watcher's broadcast subscription directly without any envelope encoding
