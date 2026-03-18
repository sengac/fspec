@GIT-040
Feature: Replace diff-worker.ts with native Rust NAPI diff operations
  """
  Rust side: Add get_checkpoint_file_diff(dir, filepath, checkpoint_ref) to codelet-git/src/diff.rs. Use gix to resolve the ref, read blob content from both commits (checkpoint tree and HEAD tree), then use similar::TextDiff to generate unified diff.
  NAPI side: Add #[napi] pub fn get_checkpoint_file_diff(dir, filepath, checkpoint_ref) to codelet/napi/src/git.rs, wrapping the new Rust function.
  TypeScript side: In diff.ts, replace getCheckpointFileDiff to call new NAPI binding instead of execSync('git show'). Remove Worker imports from FileDiffViewer.tsx and CheckpointViewer.tsx — call getFileDiff/getCheckpointFileDiff directly in useEffect with synchronous NAPI. Keep parseDiff/DiffLine as-is.
  Cleanup: Delete src/git/diff-worker.ts, src/git/worker-path.ts, src/tui/components/__tests__/worker-path-resolution.test.tsx. Remove esbuild diff-worker step from package.json 'build' script. The feature file for BUG-071 can remain as historical documentation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All diff operations (working-dir vs HEAD, checkpoint vs HEAD) MUST use gitoxide — no git CLI (execSync/spawn) allowed
  #   2. No worker_threads usage for diff — direct synchronous NAPI calls replace the Worker pattern
  #   3. The diff-worker.js esbuild step MUST be removed from the build pipeline (package.json)
  #   4. Diff output format from Rust MUST match the existing unified diff format (lines prefixed with +/-/space, with header lines) so diff-parser.ts continues to work unchanged
  #   5. Binary files MUST be detected and return '[Binary file - no diff available]' instead of binary content
  #   6. Files that exist in HEAD but not in the checkpoint MUST return a descriptive 'Will be deleted on restore' message
  #   7. Large diffs MUST be truncated at 20,000 lines with a truncation message
  #   8. The SEA binary MUST be able to show diffs without any external files or git binary
  #   9. Deleted files: diff-worker.ts, worker-path.ts, worker-path-resolution.test.tsx, and the BUG-071 feature file should be cleaned up
  #
  # EXAMPLES:
  #   1. User selects a modified file in FileDiffViewer → NAPI getFileDiff(cwd, filepath) is called directly → unified diff string returned → parseDiff() renders it with colored +/- lines
  #   2. User selects a file in CheckpointViewer → NAPI getCheckpointFileDiff(cwd, filepath, checkpointRef) is called → shows what will change on restore with +/- colored lines
  #   3. User selects a file in CheckpointViewer that doesn't exist in the checkpoint → sees 'Will be deleted on restore' message
  #   4. SEA binary user opens diff viewer → diffs load without MODULE_NOT_FOUND error (no diff-worker.js needed)
  #   5. User views diff of a binary file (e.g. .node, .png) → sees '[Binary file - no diff available]' instead of garbled content
  #   6. npm run build completes without any esbuild diff-worker step — only Vite bundle + NAPI build
  #   7. User views diff of a very large file (>20,000 lines) → diff is truncated with '[File truncated]' message
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have diff operations run natively in Rust via NAPI
    So that the SEA binary works without external diff-worker.js and no git CLI dependency remains

  @rust
  @napi
  Scenario: Working directory diff via NAPI returns unified diff
    Given a git repository with a tracked file that has uncommitted changes
    When the NAPI getFileDiff function is called with the repository path and file path
    Then it returns a unified diff string with lines prefixed by "+", "-", or " "
    And the diff header contains line count information
    And the result is identical in format to the previous TypeScript implementation

  @rust
  @napi
  Scenario: Checkpoint file diff via NAPI returns restore preview
    Given a git repository with a ghost checkpoint containing a different version of a file
    When the NAPI getCheckpointFileDiff function is called with the repository path, file path, and checkpoint ref
    Then it returns a unified diff comparing HEAD content to checkpoint content
    And the diff shows what will change when the checkpoint is restored

  @rust
  @napi
  Scenario: Checkpoint diff for file not in checkpoint returns deletion message
    Given a git repository with a ghost checkpoint
    And a file that exists in HEAD but not in the checkpoint
    When the NAPI getCheckpointFileDiff function is called for that file
    Then it returns a message containing "Will be deleted on restore"

  @rust
  @napi
  Scenario: Binary file diff returns binary indicator
    Given a git repository with a binary file that has uncommitted changes
    When the NAPI getFileDiff function is called for the binary file
    Then it returns "[Binary file - no diff available]"

  @rust
  @napi
  Scenario: Large diff is truncated at 20000 lines
    Given a git repository with a file that produces more than 20000 diff lines
    When the NAPI getFileDiff function is called for that file
    Then the diff output contains no more than 20000 content lines
    And the output ends with a "[File truncated" message

  @tui
  @integration
  Scenario: FileDiffViewer uses direct NAPI call instead of worker thread
    Given the FileDiffViewer component is mounted with a list of changed files
    When a file is selected for diff viewing
    Then the diff is loaded via a direct NAPI call without using worker_threads
    And no Worker instance is created
    And the parsed diff lines are displayed with colored +/- indicators

  @tui
  @integration
  Scenario: CheckpointViewer uses direct NAPI call instead of worker thread
    Given the CheckpointViewer component is mounted with checkpoint data
    When a checkpoint file is selected for diff viewing
    Then the checkpoint diff is loaded via a direct NAPI call without using worker_threads
    And no Worker instance is created
    And the restore preview is displayed with colored +/- indicators

  @sea
  @integration
  @build
  Scenario: Build pipeline does not include esbuild diff-worker step
    Given the package.json build script
    When "npm run build" is executed
    Then the build does not run any esbuild command for diff-worker.ts
    And the build succeeds with only the Vite bundle and NAPI build steps
    And no dist/git/diff-worker.js file is produced

  @cleanup
  Scenario: Legacy diff-worker files are removed from codebase
    Given the fspec source tree
    Then src/git/diff-worker.ts does not exist
    And src/git/worker-path.ts does not exist
    And src/tui/components/__tests__/worker-path-resolution.test.tsx does not exist
    And no source file imports from "worker-path" or "diff-worker"
