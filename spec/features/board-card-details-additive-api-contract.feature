@BOARD-024
@board
@rpc
@tui
@rust
@integration
@phase-1
@critical
Feature: Board card details additive API contract
  """
  BOARD-024 adds only get_work_unit_details after every frozen RPC and backend member. Header attributes, public signatures, default bodies, implementation bodies and original ordering remain exact. Frozen REFAC-019 snapshots are read-only evidence, not regenerated expectations. The appended RPC returns Result<String,String> containing the id/workspace/workUnit JSON envelope; backend errors use BackendError::WorkUnitDetails. Actual compose chains are resolved before AST inspection. MCP registration remains an exact service-method set with strict read-only arguments. These acceptance tests compile before the new method exists.
  """

  Background: User Story
    As a developer extending the board details capability
    I want the original public and wire contracts preserved exactly
    So that the additional canonical read does not change existing consumers or wire variants

  Scenario: Canonical detail API appends exactly one method without changing frozen prefixes
    Given the frozen original RPC and backend contract snapshots and actual composed sources
    When the canonical details method is inspected on the RPC and backend surfaces
    Then every frozen header and ordered member is unchanged and only the exact expected details tail is added

  Scenario: Canonical detail MCP registration matches the service with strict read-only arguments
    Given the actual composed service declaration and typed MCP registry
    When method sets flags and canonical detail arguments are checked
    Then the registry equals the service exactly and the read-only details method rejects missing invalid and unknown arguments
