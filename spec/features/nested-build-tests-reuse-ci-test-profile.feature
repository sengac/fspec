@done
@BUG-574
@BUG-317
@session
@testing
Feature: Self-building shape tests reuse the ci-test build
  """
  sessions/tests/skeleton_invariants.rs, rpc076_session_manager_handle_imports_shape.rs,
  session_manager_shape.rs and rpc-types/tests/bug146_napi_attribute_scoping.rs spawn nested cargo
  build/clippy/test commands. Without an explicit profile those build a second, dev-profile
  dependency tree beside the outer ci-test one (>20 GB), which does not fit on the development
  machine. Every nested build passes --profile ci-test (release builds stay explicit), and the
  stale RPC-041 assertions in session_manager_shape follow the current module layout.
  BUG-574: the rule covers cargo check too and every test file of the workspace, so the
  codelet_graph_crate_lift check/test/clippy calls (and any new nested call) cannot build
  the dev profile again.
  """

  Background: User Story
    As an fspec developer
    I want the self-building shape tests to reuse the outer test build
    So that they run on the development machine without exhausting the disk

  Scenario: Nested cargo build invocations select the ci-test profile
    Given every test file under rust/*/tests
    When every nested cargo invocation is inspected
    Then each build, check, clippy or test invocation passes --profile ci-test or --release

  Scenario: A nested cargo call without a profile is reported
    Given a test source that runs cargo test without --profile
    When its nested cargo invocations are inspected
    Then the check reports the file and the "test" subcommand
