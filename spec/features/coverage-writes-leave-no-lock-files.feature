@done
@coverage-tracking
@cli
@rust
@BUG-551
Feature: Coverage lock files are left behind in spec/features
  """
  SidecarTransaction (fspec-core coverage_lineage/transaction.rs) keeps its name and call sites but holds crate::io::spec_lock::lock_spec_dir for the project root derived from the sidecar path (the parent of the nearest 'features' ancestor), so coverage writes and spec JSON writes share one reentrant lock. REFAC-020 tests that held the sidecar .lock now hold spec/.fspec-write.lock.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A coverage sidecar write (generate-coverage, link-coverage, unlink-coverage, add-scenario, update-scenario, delete-scenario, delete-scenarios) holds the project's spec write lock (spec/.fspec-write.lock, BUG-477) instead of a per-feature <name>.feature.coverage.lock
  #   2. The lock file must persist for correctness: deleting a flock file after unlocking lets a waiter lock the unlinked inode while a newcomer locks a new file, so two writers run at once. So there is ONE persistent lock per spec root, never one per feature
  #   3. A crashed holder never blocks or accumulates: the OS releases a flock when its process exits, and the single lock file is reused
  #   4. Existing *.feature.coverage.lock leftovers are removed with a one-off script in scripts/; fspec carries no legacy cleanup code, and fspec's own .gitignore drops the per-feature rule
  #
  # EXAMPLES:
  #   1. After link-coverage and generate-coverage on a project, spec/features contains no *.coverage.lock file and spec/ contains only .fspec-write.lock
  #   2. While another process holds spec/.fspec-write.lock, link-coverage waits and writes the sidecar only after the lock is released
  #   3. scripts/remove-coverage-lock-files.sh run in a project deletes every spec/features/**/*.feature.coverage.lock and nothing else
  #
  # ========================================
  Background: User Story
    As a developer using fspec in a project
    I want to have coverage writes leave no lock files beside the coverage files
    So that my spec folder and git status stay free of clutter

  Scenario: Coverage commands leave no lock file beside the coverage files
    Given a project with a feature and its coverage file
    When add-scenario, link-coverage, unlink-coverage and generate-coverage rewrite the coverage file
    Then spec/features and spec contain no lock file

  Scenario: A coverage write waits for the spec write lock
    Given another process holds spec/.fspec-write.lock
    When link-coverage runs
    Then it writes the coverage file only after that process releases the lock

  Scenario: The cleanup script removes leftover coverage lock files
    Given a project whose spec/features holds leftover .feature.coverage.lock files beside features, coverage files and other files
    When scripts/remove-coverage-lock-files.sh runs in the project
    Then every .feature.coverage.lock file is gone and every other file is kept
