@cli
@rpc
@integration
@tool-execution
@BUG-424
Feature: Stale approval replies are rejected atomically
  """
  Split from client-mcp-full-rpc.feature so each feature has one test file:
  rust/fspec/tests/client_mcp_approval_regressions.rs drives
  SessionManagerHandle::reply_interaction_checked directly (no agent loop or
  model call). BUG-424 removed that test file's dead env! redirect for its
  #[path] fixtures.
  """

  Background: User Story
    As an MCP client answering daemon approvals
    I want a reply to an interaction that has since changed to be rejected
    So that my answer never lands on a newer prompt I have not seen

  Scenario: Reject stale approval responses atomically
    Given a pending daemon interaction
    When that interaction changes before its checked response is dispatched
    Then the response is rejected without answering the newer interaction
