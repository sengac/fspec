@done
@rpc-089
@rust
@mcp
@agent-loop
@regression
@source-shape
@RPC-089
Feature: Agent loop: MCP injection drain (mcp_injection_rx tokio::select! arm + mcp_channel_open flag)
  """
  Pattern mirrors RPC-082/083/084/085/086/088 regression-shape coverage: read agent_loop.rs as a string, brace-balance to scope assertions to the function body, byte-offset ORDER for sequencing invariants
  Implementation already exists in rust/agent-loop/src/agent_loop.rs:74-243 (function signature line 74-77, mcp_channel_open flag line 83, tokio::select! arm lines 194-242) — this card is coverage-only structural pinning so a regression breaks the test before reaching CI
  Test file: rust/agent-loop/tests/rpc089_mcp_injection_drain.rs — integration test that reads agent_loop.rs from CARGO_MANIFEST_DIR-relative path; sub-millisecond execution; pairs with the existing single behavioural scenario in spec/features/agent-loop-mcp-injection.feature
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The canonical `agent_loop` function in `rust/agent-loop/src/agent_loop.rs` MUST declare a parameter `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` so MCP server messages can flow into the per-session loop
  #   2. The body of `agent_loop` MUST declare a `let mut mcp_channel_open = true;` flag so the tokio::select! arm can be disabled once the channel closes (preventing CPU busy-loop on a permanently-ready closed receiver)
  #   3. The `tokio::select!` body inside `agent_loop` MUST contain an arm matching the substring `result = mcp_injection_rx.recv(), if mcp_channel_open =>` — the `if mcp_channel_open` guard MUST appear inline on the same arm so polling stops the moment the flag is flipped
  #   4. The `mcp_injection_rx.recv()` arm body MUST contain a `Some(McpInjection::Notification(text)) =>` match arm that converts the notification into an `InputWithImages` containing the notification text (so MCP notifications are routed as a normal LLM turn)
  #   5. The `mcp_injection_rx.recv()` arm body MUST contain a `Some(McpInjection::SamplingRequest { params, response_tx }) =>` match arm — even when the V1 implementation rejects sampling, the SamplingRequest variant MUST be acknowledged at this call site so the channel does not leak the request
  #   6. The `mcp_injection_rx.recv()` arm body MUST contain a `None =>` match arm that assigns `mcp_channel_open = false;` so the closed channel cannot busy-loop the select
  #   7. The `mcp_injection_rx` parameter MUST NOT be prefixed with an underscore (e.g. `_mcp_injection_rx`) — an underscore prefix signals the RPC-072 stub state where the channel was held open but never drained
  #
  # EXAMPLES:
  #   1. Source-shape test reads agent_loop.rs and asserts the substring `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` is present (and conversely that `_mcp_injection_rx` does NOT appear) in the function signature
  #   2. Source-shape test extracts the brace-balanced body of `agent_loop` and asserts it contains `let mut mcp_channel_open = true;`
  #   3. Source-shape test extracts the `agent_loop` body and asserts the substring `result = mcp_injection_rx.recv(), if mcp_channel_open =>` is present — the inline guard on the same arm is the busy-loop prevention contract
  #   4. Source-shape test extracts the `agent_loop` body and asserts all three required match arms are present: `Some(McpInjection::Notification(text)) =>`, `Some(McpInjection::SamplingRequest { params, response_tx }) =>`, and the bare `None =>` arm
  #   5. Source-shape test verifies byte-offset ORDER: the `mcp_channel_open = false;` assignment appears AFTER the `mut mcp_channel_open = true;` initialiser, AND inside the same `agent_loop` body — proving the flag flip lives in the None arm and the initialiser remains the canonical starting state
  #
  # ========================================
  Background: User Story
    As a agent maintainer
    I want to pin the structural shape of the MCP injection drain inside the canonical agent_loop (`mcp_injection_rx` parameter, `mcp_channel_open` flag, the tokio::select! arm gated on the flag, Notification/SamplingRequest match arms, and the None arm that flips the flag to prevent busy-loop spin)
    So that a future refactor cannot silently drop the drain or break the busy-loop guard and turn MCP server messages into dead packets without breaking CI

  Scenario: agent_loop signature declares mut mcp_injection_rx parameter (no underscore prefix)
    Given I read the source of rust/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "mut mcp_injection_rx: mpsc::Receiver<McpInjection>"
    And the source must NOT contain "_mcp_injection_rx"

  Scenario: agent_loop body declares the mcp_channel_open initialiser
    Given I read the source of rust/agent-loop/src/agent_loop.rs
    When I extract the brace-balanced body of the "fn agent_loop" function
    Then the function body must contain "let mut mcp_channel_open = true;"

  Scenario: tokio::select! arm is gated on if mcp_channel_open
    Given I read the source of rust/agent-loop/src/agent_loop.rs
    When I extract the brace-balanced body of the "fn agent_loop" function
    Then the function body must contain "result = mcp_injection_rx.recv(), if mcp_channel_open =>"

  Scenario: agent_loop body matches all three McpInjection outcomes
    Given I read the source of rust/agent-loop/src/agent_loop.rs
    When I extract the brace-balanced body of the "fn agent_loop" function
    Then the function body must contain "Some(McpInjection::Notification(text)) =>"
    And the function body must contain "Some(McpInjection::SamplingRequest { params, response_tx }) =>"
    And the function body must contain "mcp_channel_open = false;"

  Scenario: mcp_channel_open initialiser precedes the flag flip in agent_loop
    Given I read the source of rust/agent-loop/src/agent_loop.rs
    When I extract the brace-balanced body of the "fn agent_loop" function
    Then the function body must contain "let mut mcp_channel_open = true;"
    And the function body must contain "mcp_channel_open = false;"
    And the offset of "let mut mcp_channel_open = true;" must be less than the offset of "mcp_channel_open = false;"
