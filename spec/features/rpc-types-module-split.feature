@REFAC-021
@rpc
@refactor
@code-quality
Feature: rpc-types split into per-surface modules
  """
  rust/rpc-types/src/lib.rs keeps only module declarations, `pub use` re-exports and the crate
  docs. Each wire surface (git state, checkpoints, sessions, chrome, providers, credentials,
  blocklist, worktrees, scheduling, stream chunks, per-session state, pause/HITL, workspaces) lives
  in its own module that imports what it needs from the crate root, and the JSON round-trip suite
  lives in its own test module. Source-shape tests that read rpc-types read the whole module tree
  through codelet_test_helpers::source_composition::read_module_tree.
  """

  Background: User Story
    As an fspec developer
    I want rpc-types split into small modules
    So that every wire type is easy to find and no file exceeds 300 lines

  Scenario: Every rpc-types source file stays under 300 lines
    Given the rust/rpc-types/src directory
    When the line count of every Rust source file is measured
    Then no file has 300 or more lines

  Scenario: The public surface is unchanged
    Given code that names WorkUnitInfo, SessionInfo, StreamChunk, GitState, WorkspaceEntry and WorkspaceWorkUnits through codelet_rpc_types
    When it is compiled and the values round-trip through JSON
    Then each value deserializes back equal to the original
