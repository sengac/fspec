@done
@feature-management
@cli
@RPC-218
Feature: Port delete-features command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/delete_features.rs reuses crate::io::feature_glob::glob_feature_files for the recursive spec/features walk (relative forward-slash paths, alphabetical sort); a DirectoryNotFound result is mapped to an empty list to preserve the TS 'No feature files found' message.
  Feature-level tags come from parse_feature_lenient(feature.tags) with the leading '@' re-prepended (gherkin-0.16 strips it); AND match via tags.iter().all(); unparseable/featureless files skipped. Output JSON envelope {success, deletedCount, message?, files?, error?}; CLI bridge owns all rendering (dry-run/real/empty) and marshals repeatable --tag into {tags:[...], dryRun?}.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An empty tag list MUST return success=false, deletedCount=0, error 'At least one --tag is required'
  #   2. When there are no feature files (or no spec/features dir), MUST return success=true, deletedCount=0, message 'No feature files found'
  #   3. Matching uses AND logic on FEATURE-level tags only: a file matches when every supplied tag is present in feature.tags (compared with leading '@')
  #   4. Feature files with invalid Gherkin or no Feature are silently skipped during matching
  #   5. When no files match, MUST return success=true, deletedCount=0, message 'No feature files found matching tags'
  #   6. With dryRun=true, NO files are deleted; MUST return success=true, deletedCount=N, files=[...], message 'Would delete N feature file(s)'
  #   7. Without dryRun, every matching file is unlinked; MUST return success=true, deletedCount=N, files=[...], message 'Deleted N feature file(s)'
  #   8. Returned file paths are relative (forward-slash) paths under spec/features/; coverage sidecars are NOT deleted
  #   9. CLI dry-run prints 'Dry run mode - no files modified', a 'Would delete N feature file(s):' header, and each file as '  - <file>'; real delete prints '✓ <message>' then 'Deleted files:' and each '  - <file>'; exit 0
  #
  # EXAMPLES:
  #   1. Dispatcher with tags=['@deprecated'] over 3 features (2 tagged @deprecated) returns deletedCount=2 and removes those 2 files
  #   2. Dispatcher with tags=['@critical','@spike'] (AND logic) only matches features carrying BOTH tags
  #   3. Dispatcher with dryRun=true returns deletedCount + files but leaves all files on disk
  #   4. Dispatcher with an empty tags array returns success=false, error 'At least one --tag is required'
  #   5. Dispatcher with a tag no feature carries returns success=true, deletedCount=0, message 'No feature files found matching tags'
  #   6. CLI: `fspec delete-features --tag @deprecated --dry-run` exits 0, prints 'Dry run mode - no files modified' and the would-delete list
  #   7. CLI: `fspec delete-features --tag @deprecated` exits 0, prints '✓ Deleted N feature file(s)' and the deleted files list
  #   8. CLI with no --tag exits 1 with stderr 'Error:' prefix
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to bulk-delete feature files whose feature-level tags match ALL of the supplied tags (with a dry-run preview) via both the LLM dispatcher and the shell CLI
    So that I can clean up obsolete or completed-phase features with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: CLI dry-run previews deletions without removing files
    Given a tempdir with two features tagged @deprecated
    When I run 'fspec delete-features --tag @deprecated --dry-run' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring 'Dry run mode - no files modified'
    And stdout contains the substring 'Would delete 2 feature file(s):'
    And both feature files still exist on disk

  Scenario: CLI deletes matching features and lists them
    Given a tempdir with two features tagged @deprecated
    When I run 'fspec delete-features --tag @deprecated' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Deleted 2 feature file(s)'
    And stdout contains the substring 'Deleted files:'
    And both feature files no longer exist on disk

  Scenario: CLI with no --tag exits 1 with stderr Error prefix
    Given a tempdir with a feature tagged @deprecated
    When I run 'fspec delete-features' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec delete-features --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/delete-features.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with two features tagged @deprecated
    When I run a dry-run delete-features once via the dispatcher and once via the CLI on identical inputs
    Then both front doors report the same matching files and deletedCount
