@done
@parity
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: NAPI / TypeScript surface unaffected by RPC-008
  Cross-language invariant: RPC-008 lands a brand-new Rust crate
  (rust/fspec-tui) without touching any NAPI or TypeScript source
  file. The existing Vitest smoke test for the WorkUnitInfo NAPI shape
  must remain byte-equal to its RPC-005 baseline.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want RPC-008 to land without modifying any NAPI / TypeScript source file and the existing napi-workunitinfo-shape Vitest test to remain green
    So that the TS frontend stays fully decoupled from the new Rust TUI crate and downstream consumers (npm test, CI) see no regression

  Scenario: The Vitest smoke test for WorkUnitInfo shape remains green
    Given the existing test src/__tests__/napi-workunitinfo-shape.test.ts
    When `npm test` is run after RPC-008 lands
    Then the suite passes without modifications
    And no NAPI or TypeScript source file was touched by RPC-008
