@done
@rpc
@infrastructure
@session-management
@codelet
@RPC-039
Feature: Move BackgroundSession from codelet-napi into codelet-sessions, replace NAPI references
  """
  Architecture: BackgroundSession is moved into the new `background_session` module of codelet-sessions. Supporting types it OWNS by field (PromptInput, CompactionProgress, WorkUnitContext, IncomingMessage, BridgeImageData, SessionError) and helpers it consumes (format_incoming_message) move with it. The napi side keeps `pub use codelet_sessions::background_session::{..}` re-exports so the rest of session_manager.rs (ChainOfCommand, SessionManager, agent_loop, the unit-test module) keeps resolving paths it used pre-move
  Architecture: handle_output gains a `chunks_tx: Option<broadcast::Sender<(SessionId, StreamChunk)>>` field. RPC-039 leaves this field defaulted to None (BackgroundSession::new keeps its existing arg list); the NAPI shell continues to fan chunks out via the pre-existing GLOBAL_CHUNK_CALLBACK code path that LIVES OUTSIDE the moved code. RPC-041 is the card that adds the chunks_tx wiring at the SessionManager construction site and deletes GLOBAL_CHUNK_CALLBACK
  Architecture: send_input's return type narrows from `napi::bindgen_prelude::Result<()>` to `Result<(), String>`. The NAPI free function session_send_input (at codelet/napi/src/session_manager.rs:6555) which forwards into BackgroundSession::send_input is updated to map the String error back into `napi::Error::from_reason(err)` so the TS Promise<void> signature is preserved verbatim — this is the only NAPI-shell adapter change forced by the move and it MUST live in napi (NOT inside codelet-sessions)
  Architecture: `WorkUnitContext` (locally defined with optional fields + `is_set()` and `format_for_environment()` helpers) is kept as the BackgroundSession-internal type. `codelet_rpc_types::WorkUnitContext` (required fields) is the WIRE type — conversion impls between the two are deferred to RPC-042 (Implement SessionManagerHandle). Same pattern for CompactionProgress where shapes coincide today and a wire-side use is deferred
  Architecture: the attachment's reference to lines `459-1362` is slightly off — the actual BackgroundSession struct + impl span is `codelet/napi/src/session_manager.rs:459-1356`. Supporting types that must move with it (PromptInput at 93, CompactionProgress at 116, WorkUnitContext at 128, IncomingMessage at 277, BridgeImageData at 291, format_incoming_message at 342, SessionError at 428) live above the struct in the same file and total roughly 90 LOC of extra code to relocate; the unit-test module at the end of session_manager.rs (tests::test_*) continues to live in NAPI and references the re-exported symbols
  Architecture: the existing dependency-rule test at codelet/sessions/tests/skeleton_invariants.rs already enforces `no codelet-napi in transitive metadata`. This card piggy-backs on that test plus a new lightweight static-shape test (codelet/sessions/tests/background_session_shape.rs) that asserts BackgroundSession's public path resolves and that the napi adapter still re-exports it. No new dependency-rule test crates are required.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The complete BackgroundSession struct + impl block currently at codelet/napi/src/session_manager.rs lines 459-1356 lives verbatim in codelet/sessions/src/background_session.rs after the move
  #   2. No napi:: references appear inside codelet/sessions/src/background_session.rs — the only NAPI-typed return inside the moved code (napi::bindgen_prelude::Result<()> on send_input) is rewritten to Result<(), String>, and the napi::Error::from_reason(...) call site is rewritten to a plain format!-produced String error
  #   3. Inside the moved background_session.rs no `crate::persistence::` imports remain — every usage of load_session, append_message_with_metadata, update_session_tokens, MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent now resolves to codelet_core::persistence (the lifted home from RPC-031..RPC-034)
  #   4. Inside the moved background_session.rs the single `crate::types::FspecResult` reference (used by the fspec_response channel + send_fspec_result + wait_for_fspec_response) is rewritten to codelet_rpc_types::FspecResult (lifted in RPC-036)
  #   5. Supporting types defined inline above the BackgroundSession struct that BackgroundSession depends on at the field level (PromptInput, CompactionProgress, WorkUnitContext + impl, IncomingMessage + impl, BridgeImageData, format_incoming_message, SessionError + Display/Error/From impls) are also moved into codelet-sessions so the moved code compiles standalone; the napi side re-exports each via `pub use codelet_sessions::background_session::{..}` so the rest of session_manager.rs (ChainOfCommand, SessionManager, agent_loop helpers, tests) keeps compiling against the same paths
  #   6. `BackgroundSession::handle_output` no longer references the napi-side `GLOBAL_CHUNK_CALLBACK` directly: an optional `chunks_tx: Option<tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>>` field is added (defaulted to None when constructed via the existing `new()` signature) and handle_output fires `tx.send((self.id_as_session_id(), chunk))` when Some, with a TODO comment that RPC-041 will wire both ends. The pre-existing supervisor_broadcast.send(chunk) and output_buffer.push(chunk) calls remain unchanged so behaviour is identical when no chunks_tx is wired
  #   7. GLOBAL_CHUNK_CALLBACK + its OnceCell/unsafe-impl/wrapper stays alive inside codelet/napi/src/session_manager.rs for this card (deletion is explicitly RPC-041) — the NAPI shell wires the existing global callback fan-out by NOT supplying a chunks_tx and keeping its own pre-existing call path; once RPC-041 supplies a real broadcast Sender and subscribes to it, chunks reach JS through the new channel. The risk of dropped chunks is mitigated by leaving the pre-existing fan-out path intact
  #   8. The build invariants ALL hold after the move: `cargo build -p codelet-sessions` succeeds, `cargo build -p codelet-napi` succeeds (re-exports keep ChainOfCommand, SessionManager, and the public #[napi] surface compiling unchanged), `cargo build -p codelet-napi --release` regenerates codelet/napi/index.d.ts with NO removed or renamed TypeScript symbols, and `cargo metadata -p codelet-sessions --format-version 1` reports zero `codelet-napi` package entries in the transitive dependency graph
  #   9. All pre-existing unit tests still pass — the tests currently embedded in codelet/napi/src/session_manager.rs that exercise WorkUnitContext::new/format_for_environment, IncomingMessage::new/with_images, format_incoming_message, and parse_interjection continue to compile (referring to the re-exported symbols) and continue to assert the same behaviour. The codelet/sessions/tests/smoke.rs crate_compiles test continues to pass.
  #   10. No behavioural changes are introduced — this is purely a code move with import-path rewrites. The send_input return-type widening from napi::Result<()> to Result<(), String> is the only API signature change inside BackgroundSession; the NAPI free function session_send_input(...) -> napi::Result<()> at session_manager.rs:6555 maps the new String error back to napi::Error::from_reason at the wire boundary so the TypeScript shape `Promise<void>` is preserved verbatim
  #
  # EXAMPLES:
  #   1. Developer runs `cargo build -p codelet-sessions` — build succeeds and the resulting library exposes `codelet_sessions::background_session::BackgroundSession`
  #   2. Developer runs `cargo build -p codelet-napi` (default features) — build succeeds because codelet/napi/src/session_manager.rs now `pub use codelet_sessions::background_session::{BackgroundSession, IncomingMessage, BridgeImageData, format_incoming_message, PromptInput, CompactionProgress, WorkUnitContext, SessionError};` keeps every existing import path resolvable
  #   3. Developer greps `codelet/sessions/src/background_session.rs` for the regex `napi::|use napi|#\[napi` and finds zero matches — the moved file has no NAPI references
  #   4. Developer greps `codelet/sessions/src/background_session.rs` for the regex `crate::persistence|crate::types` and finds zero matches — all such imports were rewritten to codelet_core::persistence and codelet_rpc_types respectively
  #   5. Developer inspects `BackgroundSession::send_input` in the new home — its signature is `pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>` (no napi::Result), and the error path constructs `format!("Failed to send input: {}", e)` instead of `napi::Error::from_reason(...)`
  #   6. Developer inspects `BackgroundSession::handle_output` in the new home — it still pushes the chunk into output_buffer, still calls supervisor_broadcast.send(chunk.clone()), and now additionally calls `if let Some(tx) = &self.chunks_tx { let _ = tx.send((SessionId::from(self.id.to_string()), chunk.clone())); }`; there is no reference to GLOBAL_CHUNK_CALLBACK in the moved code
  #   7. Developer runs `cargo metadata -p codelet-sessions --format-version 1 | jq '.packages[].name'` and the output contains zero entries equal to `codelet-napi` — no transitive napi dependency
  #   8. Developer runs `cargo test -p codelet-napi --lib session_manager::tests` (the existing in-file test suite that includes WorkUnitContext tests, IncomingMessage tests, format_incoming_message tests, parse_interjection tests) — every test passes via the re-exported types
  #   9. Developer runs `cargo test -p codelet-sessions --tests` — the existing smoke test crate_compiles passes, plus a new integration test (RPC-039 specific) that asserts BackgroundSession can be statically referenced as `codelet_sessions::background_session::BackgroundSession` (proves the type is publicly accessible from the new home)
  #   10. Developer runs `cargo build -p codelet-napi --release` regenerating codelet/napi/index.d.ts — git diff shows no changes (or only whitespace/comment changes) compared to the pre-move version, proving the NAPI public surface is byte-stable
  #
  # ========================================
  Background: User Story
    As a Rust developer porting BackgroundSession from codelet-napi
    I want to move the entire BackgroundSession struct and impl into the NAPI-free codelet-sessions crate with import-path rewrites only
    So that the agent loop becomes independent of codelet-napi so the fspec binary, codelet-fspec, and future SessionManagerHandle implementation can consume it without inheriting a transitive NAPI dependency

  Scenario: codelet-sessions builds standalone with BackgroundSession at its new home
    Given the BackgroundSession struct and impl have been moved into codelet/sessions/src/background_session.rs
    When I run `cargo build -p codelet-sessions`
    Then the build completes successfully with no errors
    And the public path `codelet_sessions::background_session::BackgroundSession` resolves to the moved struct

  Scenario: codelet-napi still builds against the re-exported BackgroundSession
    Given codelet/napi/src/session_manager.rs now `pub use`s BackgroundSession + its companion types from codelet-sessions
    When I run `cargo build -p codelet-napi`
    Then the build completes successfully with no errors
    And the rest of session_manager.rs (ChainOfCommand, SessionManager, the agent_loop, the #[napi] free functions, the unit-test module) resolves every BackgroundSession / PromptInput / IncomingMessage / BridgeImageData / format_incoming_message / WorkUnitContext / CompactionProgress / SessionError path through the re-exports

  Scenario: The moved background_session.rs has no napi:: references
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I grep the moved file for the regex `napi::|use napi|#\[napi`
    Then I find zero matches in codelet/sessions/src/background_session.rs

  Scenario: The moved background_session.rs has no crate::persistence or crate::types imports
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I grep the moved file for the regex `crate::persistence|crate::types`
    Then I find zero matches in codelet/sessions/src/background_session.rs
    And every persistence import resolves to codelet_core::persistence
    And every FspecResult reference resolves to codelet_rpc_types::FspecResult

  Scenario: send_input is rewritten to a non-NAPI Result type
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I inspect the `send_input` method signature in the moved file
    Then the signature is `pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>`
    And the error construction site uses `format!("Failed to send input: {}", e)` (not `napi::Error::from_reason(...)`)
    And the napi-side free function session_send_input maps the new String error back to napi::Error::from_reason at the wire boundary so the TypeScript Promise<void> signature is preserved

  Scenario: handle_output uses the new chunks_tx broadcast and no longer touches GLOBAL_CHUNK_CALLBACK
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I inspect the `handle_output` method in the moved file
    Then it still pushes the chunk into output_buffer
    And it still calls `supervisor_broadcast.send(chunk.clone())`
    And it additionally calls the new chunks_tx broadcast (`if let Some(tx) = &self.chunks_tx { let _ = tx.send((<session-id>, chunk.clone())); }`)
    And there is zero reference to GLOBAL_CHUNK_CALLBACK in the moved file
    And the GLOBAL_CHUNK_CALLBACK global itself still lives in codelet/napi/src/session_manager.rs (deletion is explicitly deferred to RPC-041)

  Scenario: codelet-sessions has no transitive dependency on codelet-napi
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I run `cargo metadata -p codelet-sessions --format-version 1`
    Then the resulting JSON contains zero packages with name `codelet-napi`

  Scenario: Pre-existing in-file unit tests in codelet-napi still pass via the re-exports
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    And codelet/napi/src/session_manager.rs re-exports BackgroundSession, IncomingMessage, BridgeImageData, format_incoming_message, PromptInput, CompactionProgress, WorkUnitContext, SessionError from codelet-sessions
    When I run `cargo test -p codelet-napi --lib session_manager::tests`
    Then every pre-existing test (WorkUnitContext::new tests, format_for_environment tests, IncomingMessage::new/with_images tests, format_incoming_message tests, parse_interjection tests) passes with status ok

  Scenario: codelet-sessions tests assert BackgroundSession is reachable from its new home
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    And the integration test codelet/sessions/tests/background_session_shape.rs has been added
    When I run `cargo test -p codelet-sessions --tests`
    Then the existing smoke test `crate_compiles` passes with status ok
    And the new shape test asserts the path `codelet_sessions::background_session::BackgroundSession` resolves at compile-time
    And the shape test asserts the supporting types (PromptInput, CompactionProgress, WorkUnitContext, IncomingMessage, BridgeImageData, SessionError) are publicly reachable from the same module
    And the shape test asserts BackgroundSession::send_input returns `Result<(), String>` (not napi::Result)

  Scenario: NAPI TypeScript surface is byte-stable across the move
    Given the BackgroundSession code has been moved into codelet/sessions/src/background_session.rs
    When I run `cargo build -p codelet-napi --release` regenerating codelet/napi/index.d.ts
    Then no TypeScript interface, function, enum, or type alias in the regenerated codelet/napi/index.d.ts is removed
    And no TypeScript interface, function, enum, or type alias is renamed
    And no field of any TypeScript interface is reordered, renamed, or removed
