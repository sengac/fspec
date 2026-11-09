@done
@test-coverage
@codelet
@testing
@COV-061
Feature: Every Rust source in the workspace belongs to a Cargo package
  """
  rust/Cargo.toml is a virtual workspace with no root package, so a .rs file that sits directly
  under rust/tests or rust/examples belongs to no Cargo target: it is never compiled and its
  assertions are never checked. COV-060 deleted one such file; COV-061 removed the remaining 29
  rust/tests files, 6 rust/examples files and the placeholder rust/benches/token_tracking.rs (19 were stale copies of tests that already live
  in a crate, the rest exercised the removed `codelet` crate or features that no longer exist).
  This scenario keeps the workspace free of unbuilt sources.
  """

  Background: User Story
    As a developer reading the fspec test suite
    I want every Rust test file to be built by some Cargo target
    So that no assertion in the repository is silently unverified

  Scenario: No Rust source lies outside a Cargo package
    Given the rust/ workspace whose root manifest is a virtual workspace
    When every .rs file outside rust/target is checked for an enclosing Cargo package
    Then each file lies in a workspace member package or a patched crate
    And rust/tests and rust/examples contain no Rust sources
