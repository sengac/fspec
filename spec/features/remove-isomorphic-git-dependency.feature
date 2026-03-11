@GIT-039
Feature: isomorphic-git not fully removed - still used in production and test code

  """
  Replaces isomorphic-git with gitoxide NAPI-RS bindings from @sengac/codelet-napi. Adds resolveRef, gitInit, gitAdd, gitCommit, gitSetConfig to codelet/git Rust crate and codelet/napi NAPI bindings. Removes obsolete stash loading from fspecStore.ts (ghost commits replaced stashes). Updates CheckpointViewer.tsx to use NAPI resolveRef. Migrates universal-test-setup.ts to NAPI bindings for git repo initialization.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ALL git operations MUST use gitoxide NAPI-RS bindings (@sengac/codelet-napi) - no isomorphic-git imports anywhere
  #   2. isomorphic-git MUST be removed from package.json dependencies and build script externals
  #   3. Missing NAPI bindings (resolveRef, init, add, commit, setConfig) MUST be added to codelet/git and codelet/napi before removing isomorphic-git
  #   4. Test infrastructure (universal-test-setup.ts) MUST use NAPI bindings for git repo setup - no isomorphic-git in tests
  #   5. Documentation and feature files MUST be updated to reference gitoxide/NAPI-RS instead of isomorphic-git
  #   6. The stash loading in fspecStore.ts (git.log refs/stash) is obsolete since checkpoints now use ghost commits - should be removed or replaced with ghost checkpoint listing
  #   7. Build and all existing tests MUST pass after the migration with zero isomorphic-git references in source
  #
  # EXAMPLES:
  #   1. CheckpointViewer.tsx calls git.resolveRef() at 3 sites - after adding resolveRef NAPI binding, these calls switch to NAPI and the import is removed
  #   2. fspecStore.ts loadStashes() uses git.log(refs/stash) - since ghost commits replaced stashes, this code is dead and should be removed
  #   3. universal-test-setup.ts uses isomorphic-git for git.init/add/commit - after adding these NAPI bindings, all 19 test files stop depending on isomorphic-git
  #   4. After npm uninstall isomorphic-git, npm run build succeeds and npm test passes with no missing module errors
  #
  # ========================================

  Background: User Story
    As a developer
    I want to remove all isomorphic-git usage from the codebase
    So that all git operations use the gitoxide NAPI-RS bindings exclusively with no leftover JavaScript git library

  Scenario: CheckpointViewer uses NAPI resolveRef instead of isomorphic-git
    Given CheckpointViewer.tsx imports isomorphic-git for git.resolveRef at 3 call sites
    When the resolveRef NAPI binding is added and CheckpointViewer is updated
    Then CheckpointViewer has no isomorphic-git imports
    Then all 3 resolveRef calls use the NAPI binding from @sengac/codelet-napi


  Scenario: Obsolete stash loading removed from fspecStore
    Given fspecStore.ts imports isomorphic-git for loadStashes using git.log on refs/stash
    When the obsolete stash loading code is removed
    Then fspecStore.ts has no isomorphic-git imports
    Then the stashes state property and loadStashes action are removed from the store


  Scenario: Test infrastructure uses NAPI bindings for git operations
    Given universal-test-setup.ts uses isomorphic-git for git.init, git.add, git.commit, and git.setConfig
    When NAPI bindings for gitInit, gitAdd, gitCommit, and gitSetConfig are added and test helpers are updated
    Then no test file imports isomorphic-git
    Then all tests that set up git repositories use NAPI bindings


  Scenario: isomorphic-git dependency completely removed
    Given isomorphic-git is listed in package.json dependencies
    When all source files are migrated to NAPI bindings and isomorphic-git is uninstalled
    Then isomorphic-git is not in package.json
    Then npm run build succeeds
    Then npm test passes with no missing module errors
    Then grep finds zero isomorphic-git references in src directory
    Then the build script does not reference isomorphic-git

