@done
@diff-viewer
@critical
@tui
@git
@worker-threads
@bundling
@BUG-071
Feature: Worker module not bundled - MODULE_NOT_FOUND for diff-worker.js
  """
  Solution: Use import.meta.url and fileURLToPath to resolve worker path relative to module location. This works in both development and production environments.
  Affected files: src/tui/components/FileDiffViewer.tsx:60 and src/tui/components/CheckpointViewer.tsx:175
  Worker is already bundled separately via esbuild in package.json build script. No changes needed to build process.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Worker path must resolve relative to the fspec module location, not process.cwd()
  #   2. Worker file must be accessible when fspec is installed as a dependency in another project
  #   3. Fix must work in both development (running from source) and production (installed as dependency)
  #
  # EXAMPLES:
  #   1. When fspec is used in /Users/rquast/projects/ollama, process.cwd() returns /Users/rquast/projects/ollama, but worker is at /Users/rquast/projects/ollama/node_modules/@sengac/fspec/dist/git/diff-worker.js
  #   2. Current code in FileDiffViewer.tsx:60 uses join(process.cwd(), 'dist', 'git', 'diff-worker.js') which fails when cwd is not fspec directory
  #   3. CheckpointViewer.tsx:175 has the same issue with hardcoded process.cwd() path resolution
  #
  # ========================================
  Background: User Story
    As a developer using fspec as a dependency
    I want to spawn worker threads for git diff operations
    So that the worker module is found and loaded correctly regardless of project location

  Scenario: Worker path resolution when fspec is installed as a dependency
    Given fspec is installed as a dependency in project "/Users/rquast/projects/ollama"
    And the current working directory is "/Users/rquast/projects/ollama"
    And the worker file is located at "node_modules/@sengac/fspec/dist/git/diff-worker.js"
    When a component attempts to spawn a worker using process.cwd()
    Then the worker path should resolve to the fspec module location
    And the worker should be successfully initialized
    And no MODULE_NOT_FOUND error should occur

  Scenario: FileDiffViewer component worker initialization with correct path resolution
    Given the FileDiffViewer component is mounted
    And fspec is installed as a dependency (not running from source)
    When the component initializes the worker thread
    Then the worker path should use import.meta.url to resolve module location
    And the worker path should NOT use process.cwd()
    And the worker should successfully spawn at the correct path
    And git diff operations should work correctly

  Scenario: CheckpointViewer component worker initialization with correct path resolution
    Given the CheckpointViewer component is mounted
    And fspec is installed as a dependency (not running from source)
    When the component initializes the worker thread
    Then the worker path should use import.meta.url to resolve module location
    And the worker path should NOT use process.cwd()
    And the worker should successfully spawn at the correct path
    And checkpoint diff operations should work correctly
