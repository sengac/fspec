@done
@integration-test
@p1
@critical
@napi
@infrastructure
@rpc
@RPC-005
Feature: TS frontend NAPI WorkUnitInfo shape preserved after rpc-types lift
  """
  Architecture

  After WorkUnitInfo is lifted from rust/napi/src/types.rs into rust/rpc-types, the existing TypeScript frontend MUST continue to see the same shape with no breaking changes. This feature is a single Vitest smoke test that imports get_all_work_units via the rust/napi binding and asserts the returned object keys.

  References: rule [9] in RPC-005 (TS frontend continues to work unchanged); rule [15] (Vitest smoke test as the regression-loud canary).
  """

  Background: User Story
    As a TypeScript developer consuming rust/napi from the existing TUI
    I want a Vitest smoke test that the WorkUnitInfo shape exposed by get_all_work_units is unchanged after the rpc-types lift
    So that any future regression to the NAPI re-export pattern fails loudly in CI

  Scenario: TS frontend smoke test confirms get_all_work_units shape after the lift
    Given the WorkUnitInfo type has been lifted from rust/napi into rust/rpc-types and rust/napi has been rebuilt
    When the Vitest smoke test imports get_all_work_units from the rust/napi binding and calls it
    Then the returned value is an array whose elements have the keys id, title, workType, status, description, estimate, and epic and the existing TypeScript test suite npm test passes without modification
