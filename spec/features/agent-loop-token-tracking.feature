@done
@session-management
@RPC-086
@rust
@agent-loop
@rpc
@tokens
Feature: Agent loop emits TokenUpdate and ContextFillUpdate from BackgroundOutput
  """
  RPC-086 (child of RPC-072 family). Token tracking must call
  session.update_tokens(input, output) + session.update_reasoning_tokens(reasoning)
  from BackgroundOutput on every StreamEvent::Tokens, and emit
  StreamChunk::TokenUpdate + StreamChunk::ContextFillUpdate so the
  TUI tokens widget moves.

  After the RPC-080/RPC-081/RPC-084 ports the implementation already
  lives in the NAPI-free `codelet-agent-loop` crate; this feature
  pins the contract via structural source-string assertions and a
  small integration check against a real BackgroundSession, mirroring
  the RPC-082/083/084 coverage pattern.
  """

  Background: User Story
    As a fspec user
    I want the TUI tokens widget to update as the LLM consumes context
    So that I can see input/output token cost in real time

  Scenario: BackgroundOutput translates StreamEvent::Tokens into session updates and a StreamChunk::TokenUpdate
    Given the source of `rust/agent-loop/src/background_output.rs`
    When I locate the `StreamEvent::Tokens(info)` arm of `BackgroundOutput::emit`
    Then the arm body calls `self.session.update_tokens(info.input_tokens as u32, info.output_tokens as u32)`
    And the arm body calls `self.session.update_reasoning_tokens(r as u32)` inside an `if let Some(r) = info.reasoning_tokens` guard
    And the arm body constructs a `TokenTracker { ... }` literal populating `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `tokens_per_second`, `cumulative_billed_input`, `cumulative_billed_output`, and `reasoning_tokens`
    And the arm body returns the literal wrapped in `StreamChunk::token_update(...)`

  Scenario: BackgroundOutput translates StreamEvent::ContextFill into a StreamChunk::ContextFillUpdate
    Given the source of `rust/agent-loop/src/background_output.rs`
    When I locate the `StreamEvent::ContextFill(info)` arm of `BackgroundOutput::emit`
    Then the arm body returns `StreamChunk::context_fill_update(ContextFillInfo { ... })`
    And the `ContextFillInfo` literal populates `fill_percentage`, `effective_tokens`, `threshold`, and `context_window`

  Scenario: BackgroundSession exposes the cached token API expected by BackgroundOutput
    Given a `codelet_sessions::background_session::BackgroundSession` constructed via test helpers
    When I call `session.update_tokens(100, 50)` and then `session.update_reasoning_tokens(25)`
    Then `session.get_tokens()` returns `(100, 50, Some(25))`
    And the underlying `cached_input_tokens`, `cached_output_tokens`, and `cached_reasoning_tokens` `AtomicU32` fields hold the same values

  Scenario: codelet_rpc_types::StreamChunk declares TokenUpdate and ContextFillUpdate with matching constructors
    Given the source of `rust/rpc-types/src/lib.rs`
    When I inspect the `StreamChunk` enum
    Then the enum declares a `TokenUpdate { tokens: TokenTracker }` variant
    And the enum declares a `ContextFillUpdate { context_fill: ContextFillInfo }` variant
    And the impl block defines a `pub fn token_update(tokens: TokenTracker) -> Self` constructor returning `Self::TokenUpdate { tokens }`
    And the impl block defines a `pub fn context_fill_update(info: ContextFillInfo) -> Self` constructor returning `Self::ContextFillUpdate { context_fill: info }`

  Scenario: TokenInfo and ContextFillInfo source types carry the fields BackgroundOutput consumes
    Given the source of `rust/cli/src/interactive/output.rs`
    When I inspect the `TokenInfo` and `ContextFillInfo` structs
    Then `TokenInfo` declares `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `tokens_per_second`, and `reasoning_tokens` fields
    And `ContextFillInfo` declares `fill_percentage`, `effective_tokens`, `threshold`, and `context_window` fields
    And the `StreamEvent` enum declares both a `Tokens(TokenInfo)` and a `ContextFill(ContextFillInfo)` variant
