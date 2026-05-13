@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@napi
@rust
@tarpc
@rpc
@RPC-005
Feature: RPC architecture invariants enforced via source-shape tests
  """
  Architecture

  This feature codifies invariants that must hold across the dual-transport tarpc design:
  - codelet/rpc-embedded must accept a tokio::runtime::Handle from the host and must NOT construct its own runtime (per resolved RPC-002 Q9).
  - codelet/rpc-types is the single source of truth for shared serde types; codelet/napi re-exports them and never redefines them.
  - codelet/rpc-embedded only constructs an in-process channel transport via tarpc::transport::channel (no network serialization in embedded mode — covers the "no serialization" half of the embedded-transport scenario).
  - the shared service impl crate codelet/rpc must NOT yet depend on codelet/core or the NAPI work_units_watcher (RPC-005 rule [10]: spike uses a test-only fixture; real watcher integration is a follow-up card).
  - the rpc-server binary must bind to 127.0.0.1 only (RPC-005 rule [13]: TCP loopback only; UDS / non-loopback bind is deferred until daemon topology is decided).

  Tests are pure source-shape / cargo-check assertions; no runtime spawn.

  References: spec/attachments/RPC-002/07-recommended-architecture.md section 7; spec/attachments/RPC-002/11-open-questions-and-risks.md Q9; rules [10], [13] in RPC-005.
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want automated regression tests that fail loudly when the runtime-Handle and single-source-of-truth invariants are violated
    So that future changes cannot silently fork the type system or spawn rogue runtimes

  Scenario: EmbeddedTransport requires a tokio runtime Handle at construction
    Given the codelet/rpc-embedded crate exposes an EmbeddedTransport public constructor
    When I inspect the public type signature of the constructor and search the codelet/rpc-embedded source for runtime construction calls
    Then the constructor takes a tokio::runtime::Handle as a non-defaulted argument and the codelet/rpc-embedded source contains no calls to tokio::runtime::Builder or tokio::runtime::Runtime::new that would create a separate runtime

  Scenario: WorkUnitInfo is defined once in rpc-types and re-exported by codelet-napi
    Given codelet/rpc-types defines a public WorkUnitInfo struct with fields id, title, work_type, status, description, estimate, and epic
    When I inspect codelet/napi for definitions of WorkUnitInfo and run cargo check on the codelet workspace
    Then codelet/napi contains no struct definition named WorkUnitInfo and instead re-exports the type from codelet/rpc-types and the workspace builds successfully

  Scenario: Embedded transport uses only tarpc::transport::channel for in-process traffic
    Given codelet/rpc-embedded must perform no network serialization for in-process RPCs
    When I search the codelet/rpc-embedded source for transport constructors
    Then the only transport constructor used is tarpc::transport::channel::unbounded and no WebSocket, TCP, or bincode serialization call sites appear in the crate source

  Scenario: Shared service crate codelet-rpc has no dependency on codelet-core or the NAPI work_units_watcher
    Given the RPC-005 spike must read from a test-only in-memory fixture and not from the real codelet/core or codelet/napi work-units watcher
    When I inspect codelet/rpc/Cargo.toml dependencies and the codelet/rpc/src/ source for codelet-core or work_units_watcher imports
    Then codelet/rpc declares no codelet-core or codelet-napi dependency and the codelet/rpc source contains no use codelet_core or use codelet_napi imports

  Scenario: rpc-server binary binds to 127.0.0.1 loopback only
    Given the RPC-005 rpc-server is restricted to TCP loopback in this card
    When I inspect codelet/rpc-server/src/main.rs for the bind address literal
    Then the only bind address literal is 127.0.0.1:0 and no 0.0.0.0 or other non-loopback bind address appears in the binary source
