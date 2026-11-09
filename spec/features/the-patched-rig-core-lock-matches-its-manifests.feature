@BUG-582
@done
@rig
@providers
@testing
@rust
@test-infrastructure
@BUG-558
Feature: The patched rig-core lock matches its manifests
  """
  Changed: rust/patches/rig-core/Cargo.lock (refreshed with the documented offline procedure; only codelet-common's version changes), docs/TESTING.md (refresh after a version bump). New: rust/test-helpers/tests/bug558_rig_core_patch_lock_matches_its_manifests.rs, parsing the lock and manifests with the workspace toml crate (new dev-dependency of codelet-test-helpers). The BUG-518 test (cargo metadata --locked --offline) stays; it also failed at HEAD but depends on the local registry cache, so the exact file check is the home-independent guard.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Root cause: the release bump to 0.11.0 (0d68b62f7) changed the workspace version that codelet-common inherits, and rust/patches/rig-core/Cargo.lock still recorded codelet-common 0.10.9, so cargo test --locked refused to start; refreshing the lock offline from the workspace lock (the documented procedure) changes only that version
  #   2. The patch lock is kept current with an exact file check that needs neither cargo nor the network nor the local registry cache: for each path package in rust/patches/rig-core/Cargo.lock (rig-core and codelet-common) the locked version equals the version its manifest declares (version.workspace resolved from rust/Cargo.toml), and the locked dependency names equal the manifest's dependency names (dependencies, build-dependencies and target tables; dev-dependencies only for rig-core, the patch's root package; package renames and workspace-inherited renames resolved)
  #   3. A failing check names the package, the locked and declared versions or the missing and extra dependency names, and the documented refresh command
  #   4. docs/TESTING.md says to refresh the patch lock after a workspace version bump as well as after codelet-common gains a dependency; rust/patches/rig-core stays a standalone package outside the workspace (BUG-518 rule 0)
  #
  # EXAMPLES:
  #   1. At HEAD the patch lock records codelet-common 0.10.9 while rust/Cargo.toml's workspace version is 0.11.0: the check fails naming codelet-common, 0.10.9, 0.11.0 and the refresh command, and cargo test --manifest-path rust/patches/rig-core/Cargo.toml --locked --offline stops with the lock file needs to be updated
  #   2. After the refresh, the lock records codelet-common 0.11.0 and rig-core 0.28.0 with exactly their manifests' dependencies, and the documented command starts and runs the tests
  #   3. codelet-common gains a dependency fs3 in its Cargo.toml without a lock refresh: the check fails naming codelet-common and the missing dependency fs3
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to run the patched rig-core's tests with the command docs/TESTING.md documents
    So that the patch's tests run offline as written after every release bump

  Scenario: The patch lock records each path package at its manifest's version
    Given rust/patches/rig-core/Cargo.lock and the manifests of rig-core and codelet-common
    When the locked versions of the path packages are read
    Then rig-core is locked at the version its manifest declares
    And codelet-common is locked at the workspace version it inherits

  Scenario: The patch lock lists exactly the dependencies each path package declares
    Given rust/patches/rig-core/Cargo.lock and the manifests of rig-core and codelet-common
    When the locked dependency names of the path packages are read
    Then rig-core's locked dependencies are its dependencies, build, target and dev dependencies
    And codelet-common's locked dependencies are its dependencies, build and target dependencies

  Scenario: A stale patch lock is reported with the refresh command
    Given a patch lock that records codelet-common 0.10.9 without the dependency fs2 while the workspace version is 0.11.0
    When the lock is checked against the manifests
    Then the check fails naming codelet-common, 0.10.9, 0.11.0 and the missing dependency fs2
    And the failure gives the documented refresh command
