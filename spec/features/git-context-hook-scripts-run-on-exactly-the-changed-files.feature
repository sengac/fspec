@done
@BUG-589
@cli
@workflow-automation
@hooks
Feature: Git-context hook scripts run on exactly the changed files
  """
  Architecture notes:
  - add-virtual-hook --git-context writes spec/hooks/.virtual/<workUnitId>-<hookName>.sh (generate_virtual_hook_script in rust/fspec-core/src/commands/add_virtual_hook.rs) and stores its relative path as the hook command.
  - The hook executor (HOOK-018) runs the script in the project root with the JSON context on stdin; HOOK-019 adds stagedFiles and unstagedFiles for a gitContext hook.
  - The old script joined two space-separated lists into "$STAGED $UNSTAGED", a single space when both were empty, so its -z check never fired and the command ran with no file arguments (on the whole codebase). It also sent jq's errors to /dev/null, so a missing jq looked like "no files".
  - The script checks for jq first, reads both lists through jq's @sh quoting into one bash array, and tests the array's length. Paths reach the command as separate, exact arguments.
  - The init docs (init_doc_*.md and their fixtures) show the generated script for eslint verbatim.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The script reads the changed files as one list (stagedFiles then unstagedFiles) and checks that list's length: an empty list prints 'No changed files to process' and exits 0 without running the command
  #   2. Each changed path reaches the command as its own argument, exactly as git reported it (jq @sh quoting into a bash array), so a path with spaces is one argument
  #   3. The script still reads the stdin JSON context with jq (the executor's contract, HOOK-018); it checks for jq before anything else and, when jq is missing, prints 'jq is required to read the changed files of this --git-context hook; install jq' on stderr and exits 127 without running the command (conservative choice: keep the stdin contract rather than add a second file channel)
  #   4. jq's errors are not hidden: no 2>/dev/null, and set -euo pipefail makes a jq failure stop the script with its own message and a non-zero exit
  #   5. The init docs' 'Generated Script Example' is exactly the script add-virtual-hook generates for eslint, so the documentation cannot drift from the generator
  #
  # EXAMPLES:
  #   1. No staged or unstaged files: the script prints 'No changed files to process', exits 0 and the command does not run
  #   2. Staged src/auth.ts and 'src/my file.ts' only: the command runs once with exactly those two arguments
  #   3. Unstaged src/hooks/virtual.ts only: the command runs with exactly that argument
  #   4. jq not on PATH: the script exits 127 with the jq message on stderr and the command does not run
  #
  # QUESTIONS (ANSWERED):
  #   Q: should the generated --git-context script stop needing jq (e.g. the executor also passing the changed files another way)? Conservative choice taken: keep jq and the stdin JSON contract, fail clearly with exit 127 when jq is missing.
  #   A: Provisional, pending the product owner (helper, 2026-11-07): keep jq and the executor's stdin JSON contract; the script checks for jq first and exits 127 with a clear message when it is missing. Dropping jq needs a second channel for the file list, a product change to the HOOK-018 executor contract.
  #
  # ASSUMPTIONS:
  #   1. Provisional, pending the product owner (helper, 2026-11-07): keep jq and the executor's stdin JSON contract; the script checks for jq first and exits 127 with a clear message when it is missing. Dropping jq needs a second channel for the file list, a product change to the HOOK-018 executor contract.
  #
  # ========================================
  Background: User Story
    As a developer adding a virtual hook with --git-context
    I want to have the generated script run the command on exactly the staged and unstaged files, and say so clearly when it cannot
    So that a quality gate never silently runs on the whole codebase or passes because jq is missing

  Scenario: No changed files skips the command
    Given a --git-context hook script generated for a command that records its arguments
    When the script runs with empty stagedFiles and unstagedFiles
    Then it prints "No changed files to process" and exits 0
    And the command does not run

  Scenario: Staged files only reach the command as separate arguments
    Given a --git-context hook script generated for a command that records its arguments
    When the script runs with stagedFiles "src/auth.ts" and "src/my file.ts" and no unstagedFiles
    Then the command runs once with exactly the arguments "src/auth.ts" and "src/my file.ts"

  Scenario: Unstaged files only reach the command
    Given a --git-context hook script generated for a command that records its arguments
    When the script runs with no stagedFiles and unstagedFiles "src/hooks/virtual.ts"
    Then the command runs once with exactly the argument "src/hooks/virtual.ts"

  Scenario: A missing jq is reported instead of hidden
    Given a --git-context hook script generated for a command that records its arguments
    When the script runs with stagedFiles "src/auth.ts" and no jq on PATH
    Then it exits 127 with "jq is required to read the changed files of this --git-context hook; install jq" on stderr
    And the command does not run

  Scenario: The init docs show the generated script
    Given the script add-virtual-hook generates for the command "eslint"
    When the "Generated Script Example" of every init doc is read
    Then each example is exactly that script
