@done
@refactor
@rpc
@napi
@rust
@session-management
@persistence
@RPC-031
Feature: Lift MessageEnvelope types into codelet-core::persistence::message_envelope
  """
  MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent, TokenUsagePerMessage, ToolUseResultMetadata, ImageSource, DocumentSource, and CacheControl move into rust/core/src/persistence/message_envelope.rs. NAPI provides a thin re-export shim (`pub use codelet_core::persistence::message_envelope::*;`). The on-disk JSONL wire format (rename_all = camelCase outer wrapper, tag = type for content enums, untagged MessagePayload) is byte-identical before and after the move. The single test that references `crate::persistence::should_use_blob_storage` (which lives in napi::persistence::blob until RPC-034) is relocated to the NAPI shim's #[cfg(test)] block. Both codelet-napi and codelet-rpc-embedded now consume MessageEnvelope from codelet-core without a `rpc → napi` arrow.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All 11 envelope types live in rust/core/src/persistence/message_envelope.rs with identical serde derives, rename_all, tag attributes, and field order
  #   2. rust/napi/src/persistence/message_envelope.rs becomes a thin re-export shim (`pub use codelet_core::persistence::message_envelope::*;`) so every `crate::persistence::MessageEnvelope` import in NAPI continues to compile unchanged
  #   3. On-disk messages.jsonl format remains byte-identical (no field reorder, no rename, no #[napi] attributes added)
  #   4. The `test_blob_threshold` test referencing `crate::persistence::should_use_blob_storage` is kept in the NAPI shim's `#[cfg(test)]` block (since `should_use_blob_storage` lives in napi::persistence::blob until RPC-034)
  #
  # EXAMPLES:
  #   1. Round-trip serialization: a MessageEnvelope built with all fields populated (UUID, parentUuid, timestamp, type, provider, payload, requestId) serializes to the same JSON bytes from codelet-core as it did from codelet-napi
  #   2. An existing NAPI consumer like session_manager.rs continues to compile with `use crate::persistence::{MessageEnvelope, MessagePayload, UserMessage}` after the lift
  #   3. codelet-rpc-embedded (which already depends on codelet-core but cannot depend on codelet-napi) can now import MessageEnvelope from codelet_core::persistence::message_envelope without re-introducing a forbidden `rpc → napi` arrow
  #   4. `cargo build -p codelet-core` passes; `cargo build -p codelet-napi` passes; `cargo test -p codelet-napi persistence::tests` passes (existing 48 tests); `cargo test -p codelet-napi --test session_persistence_test` passes (23 tests)
  #
  # ========================================
  Background: User Story
    As a fspec backend engineer
    I want to lift the Claude-Code-compatible MessageEnvelope schema (MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent, TokenUsagePerMessage, ToolUseResultMetadata, ImageSource, DocumentSource, CacheControl) out of codelet-napi into codelet-core::persistence::message_envelope
    So that both the TS Ink frontend (via NAPI re-exports) and the Rust ratatui frontend (via codelet-rpc-embedded) consume the same Rust source for the on-disk JSONL wire format with byte-identical serialization

  Scenario: MessageEnvelope round-trips with byte-identical JSON from codelet-core
    Given a MessageEnvelope value constructed in codelet-core with uuid, parent_uuid, timestamp, message_type "assistant", provider "claude", an AssistantMessage payload containing one Text content, and a request_id
    When the envelope is serialized to JSON and then deserialized back
    Then the restored value equals the original
    And the JSON includes parentUuid, type, provider, message, and requestId fields with camelCase keys

  Scenario: codelet-core consumers can import MessageEnvelope without depending on codelet-napi
    Given codelet-core::persistence::message_envelope exports all 11 envelope types
    When a downstream crate that does not depend on codelet-napi (such as codelet-rpc-embedded) constructs and serializes a MessageEnvelope through codelet_core::persistence::message_envelope
    Then the build succeeds with no transitive dependency on codelet-napi
    And the produced JSON bytes match the bytes produced by codelet-napi for the same input
