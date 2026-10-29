@REFAC-019
@rpc
@integration
@code-quality
@phase-1
Feature: Track nested RPC schema declaration inputs
  """
  Keep conservative schema hashing and rerun directives sensitive to real nested
  service and type declaration inputs. Exercise the actual build script against
  an isolated filesystem. No running daemon, wire order or protocol redesign.
  This scenario is split from the refactor contract feature solely to retain
  the one-feature/one-test-file workflow invariant without changing acceptance.
  """

  Background: User Story
    As a maintainer of native RPC integration
    I want fingerprints to track extracted declaration sources
    So that changed wire inputs cannot become invisible to compatibility checks

  Scenario: Schema fingerprints track nested declaration inputs
    Given an isolated RPC workspace with nested service and type declarations
    When either nested declaration changes without changing the root files
    Then its schema fingerprint changes and Cargo rerun tracking names the input
