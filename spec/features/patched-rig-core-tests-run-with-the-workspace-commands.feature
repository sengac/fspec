@done
@BUG-518
@rig
@providers
@testing
@rust
@test-infrastructure
Feature: Patched rig-core tests run with the workspace commands
  """
  The workspace builds every crate against rust/patches/rig-core through
  [patch.crates-io]. The patch stays a standalone package outside the
  workspace members: [patch.crates-io] only applies when the patch keeps the
  upstream version (0.28.0), while UPD-001 requires every workspace member to
  carry the workspace version, so the patch keeps its own [workspace] table
  and its own Cargo.lock. That lock is generated from the workspace lock's
  versions (offline: shared dependencies at the workspace's versions, the
  rest already in the local cache), so the documented command
  cargo test --manifest-path rust/patches/rig-core/Cargo.toml --locked --offline
  runs the patch's tests without a network fetch and without rewriting the
  lock. docs/TESTING.md documents it.
  Tests: rust/test-helpers/tests/bug518_rig_core_is_a_workspace_member.rs.
  """

  Background: User Story
    As a developer verifying rig-core patch coverage
    I want to run the patched rig-core's tests offline with a documented, lock-stable command
    So that the patch's coverage can be verified without a network fetch or a rewritten lock file

  Scenario: The patched rig-core stays a standalone package outside the workspace
    Given the workspace manifest and the patched rig-core manifest
    When the workspace members and the patch's own files are read
    Then the patches/rig-core directory is not a workspace member
    And the patch declares its own workspace and keeps its own Cargo.lock

  Scenario: The patched rig-core's lock resolves offline without being rewritten
    Given the patched rig-core package and its Cargo.lock
    When cargo resolves the package with --locked and --offline
    Then resolution succeeds
    And the lock file is unchanged
