@done
@RPC-031
@persistence
@session-management
@rust
@napi
@rpc
@refactor
@critical
Feature: NAPI Re-Export Shim For Message Envelope
  """
  The NAPI persistence module retains its existing public surface (codelet_napi::persistence::*) by replacing message_envelope.rs with a 10-line re-export shim: `pub use codelet_core::persistence::message_envelope::*;`. The `test_blob_threshold` test stays in the NAPI shim's #[cfg(test)] block because it references `crate::persistence::should_use_blob_storage` (which lives in napi::persistence::blob until RPC-034). All internal NAPI modules (session_manager.rs, blob_processing.rs, napi_bindings.rs, tests.rs) continue to use `crate::persistence::{MessageEnvelope, MessagePayload, ...}` paths unchanged. Lift precedent: matches RPC-025 (history.rs) and RPC-026 (sessions.rs delete_session).
  """

  Background: User Story
    As a fspec backend engineer maintaining the NAPI surface
    I want to expose codelet_core::persistence::message_envelope types through a thin re-export shim at codelet::persistence
    So that every existing crate::persistence::* import in codelet-napi continues to compile and the on-disk JSONL wire format remains byte-identical after the lift

  Scenario: NAPI re-export shim preserves existing crate::persistence imports
    Given the NAPI persistence module has a re-export shim file that does `pub use codelet_core::persistence::message_envelope::*;`
    When an internal NAPI module writes `use crate::persistence::{MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent}`
    Then the import resolves to the codelet-core types
    And `cargo build -p codelet-napi` succeeds without modification of the importing modules

  Scenario: All NAPI persistence tests continue to pass after the lift
    Given MessageEnvelope and supporting types live in codelet-core and are re-exported by NAPI
    When the existing test suites are run with `cargo test -p codelet-core` and `cargo test -p codelet-napi`
    Then all pre-existing message_envelope serialization tests pass against the codelet-core types
    And the `test_blob_threshold` test referencing `crate::persistence::should_use_blob_storage` still passes from the NAPI shim
