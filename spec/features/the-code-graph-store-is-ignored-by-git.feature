@done
@rust
@code-analysis
@parser
@BUG-617
Feature: An indexed project shows .fspec/graph as untracked in git status
  """
  registry::WriterLock::acquire is the only place a graph directory is created (every write takes the writer lock first, KGRAPH-073), so it writes <graph dir>/.gitignore holding a comment and the single rule '*' right after create_dir_all, and rewrites it when its content differs. Queries take no writer lock and never write it. The tests run git with GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1 so no global excludes file can hide the graph.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A graph store ignores itself: every write to a graph (ast_index, reset, ast_import, learnings writes) makes the directory that holds the graph and its writer lock contain a .gitignore whose only rule is *, so git ignores every file in that directory, the .gitignore included. Nothing outside that directory changes: no project .gitignore, no fspec init or sync-version step, and it applies to any root, whatever its languages (decision on the card: the store writes its own .gitignore)
  #   2. A graph directory without that .gitignore, or with different content, gets it at its next write
  #
  # EXAMPLES:
  #   1. A git repository with one Python file and no ignore rules is indexed; git status lists only the Python file as untracked and nothing under .fspec
  #   2. A project whose .fspec/graph directory was made before this fix holds a graph and no .gitignore; ast_index runs; git status lists nothing under .fspec
  #
  # ========================================
  Background: User Story
    As a developer whose project fspec indexes
    I want to keep the code graph store out of git
    So that git status never lists the derived graph files and nobody commits them by accident

  Scenario: An indexed repository shows no graph files in git status
    Given a git repository with one Python file and no ignore rules
    When ast_index runs for the repository
    Then git status lists the Python file as untracked
    And git status lists nothing under .fspec

  Scenario: A graph directory made before the fix is ignored after its next index
    Given a git repository whose .fspec/graph directory holds a code graph and no .gitignore
    When ast_index runs for the repository
    Then git status lists nothing under .fspec
