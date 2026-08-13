@done
@RPC-059
@rust
@source-shape
@lift
@scheduler
@loop-management
Feature: Loop store lift into codelet-core::loops
  """
  Phase 7.6 of the RPC-030 roadmap. The loop_store module currently in
  rust/napi/src/scheduler/loop_store.rs MUST be lifted into
  rust/core/src/loops/mod.rs following the same lift pattern used
  by RPC-031..RPC-034 for persistence and RPC-058 for the scheduler
  engine.

  After the lift:

  - rust/core/src/loops/mod.rs hosts the LoopEntry struct, the
  LoopStore active-task manager, the IdleCheckFn type alias, and the
  process-global LOOP_STORE singleton.
  - rust/napi/src/scheduler/mod.rs collapses its
  `pub mod loop_store; pub use loop_store::LoopStore;` lines into a
  re-export shim: `pub use codelet_core::loops::{LoopEntry,
  LoopStore, IdleCheckFn};` plus an empty `pub mod loop_store {
  pub use codelet_core::loops::*; }` so existing absolute paths
  (`crate::scheduler::loop_store::LoopEntry` etc. in
  session_bindings.rs) continue to resolve unchanged.
  - The lifted module has zero `use napi` / `napi_derive` references —
  it only depends on chrono, tokio, tracing, uuid, and std (the same
  NAPI-free deps it already had).

  These tests pin the file layout so a future refactor cannot
  accidentally re-introduce a NAPI dependency on the loop store.
  """

  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the loop-store lift
    So that no future refactor can re-introduce a NAPI dependency into the loop store

  Scenario: Loop store module lives under rust/core/src/loops/
    Given the directory rust/core/src/loops/ exists
    Then it contains a file named "mod.rs"

  Scenario: codelet-core declares LoopEntry, LoopStore, and IdleCheckFn
    Given the file rust/core/src/loops/mod.rs is compiled
    Then it declares a public struct named "LoopEntry"
    And LoopEntry has fields named id, session_id, prompt, interval_seconds, created_at, expires_at, last_run_at
    And it declares a public struct named "LoopStore"
    And it declares a public type alias named "IdleCheckFn"

  Scenario: LoopStore exposes the documented async API
    Given the file rust/core/src/loops/mod.rs is compiled
    Then LoopStore declares a method named "instance" returning &'static LoopStore
    And LoopStore declares a method named "cancel" taking &str and returning a future of bool
    And LoopStore declares a method named "list_for_session" taking Uuid and returning a future of Vec<LoopEntry>
    And LoopStore declares a method named "remove_for_session" taking Uuid and returning a future of usize
    And LoopStore declares a method named "register_with_task_and_idle_check" taking LoopEntry plus on_fire and idle_check callbacks
    And LoopStore declares a method named "try_register_with_task_and_idle_check" returning Result<(), String>
    And LoopStore declares a method named "is_empty" returning a future of bool

  Scenario: The lifted loop store has no NAPI references
    Given the directory rust/core/src/loops/ exists
    Then no file under rust/core/src/loops/ contains the text "use napi"
    And no file under rust/core/src/loops/ contains the text "napi_derive"

  Scenario: rust/napi/src/scheduler/mod.rs re-exports the loops surface
    Given the file rust/napi/src/scheduler/mod.rs is compiled
    Then it contains a re-export of codelet_core::loops::LoopStore
    And it contains a re-export of codelet_core::loops::LoopEntry
    And it contains a re-export of codelet_core::loops::IdleCheckFn

  Scenario: rust/napi/src/scheduler/loop_store.rs is deleted
    Given the directory rust/napi/src/scheduler/ exists
    Then it does not contain a file named "loop_store.rs"

  Scenario: codelet-core lib.rs exports the loops module
    Given the file rust/core/src/lib.rs is compiled
    Then it declares a public module named "loops"
