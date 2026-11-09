@done
@BUG-393
@utils
@test-maintenance
@rust
Feature: Per-file line ceilings measure files, not module trees
  """
  REFAC-021 (724e13e2d) switched source-shape tests to
  source_composition::read_tree_or_file, which reads a module root as its
  whole tree; a per-file line ceiling built on it measures the wrong text
  (BUG-389 fixed source_shape_rpc054). The audit found no other ceiling that
  counts tree text: fspec-tui source_shape_rpc014/015/016/025 count the raw
  file, source_shape_rpc054 uses read_source, and the tree reads in
  agent-loop cmpct041, fspec-tui source_shape_cargo, rpc-server
  health_rpc_serverstats_rpc011 and sessions wt012 feed content checks only.
  codelet_test_helpers::source_composition::tree_measured_line_ceilings scans
  test sources for a function that enforces a line ceiling on
  read_tree_or_file / read_module_tree text (directly or through a local
  helper); a workspace guard keeps the result empty.
  Tests: rust/test-helpers/tests/bug393_line_ceilings_measure_files.rs.
  """

  Background: User Story
    As a developer keeping files under the 300-line limit
    I want to have every per-file line ceiling measure the file itself
    So that a ceiling never passes or fails on a whole module tree

  Scenario: A ceiling on module-tree text is detected
    Given a test source whose line ceiling counts read_module_tree text
    When it is scanned for tree-measured line ceilings
    Then the ceiling is reported

  Scenario: No workspace line ceiling measures a module tree
    Given the workspace test sources
    When they are scanned for tree-measured line ceilings
    Then none is found
