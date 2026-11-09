@BUG-582
@done
@HOOK-019
@cli
@workflow-automation
@hooks
Feature: Virtual hook git context and post-status hooks
  """
  Architecture notes:
  - Virtual hooks run through the command-hook executor (rust/fspec-core/src/command_hooks, HOOK-018): child process in the project root, JSON context on stdin, stdout/stderr captured, 60 second timeout.
  - update-work-unit-status runs the work unit's post-<current> hooks (leaving the status) and then its pre-<new> hooks (entering the status) before the move is saved; a failing blocking hook blocks the move.
  - A gitContext hook's context adds stagedFiles and unstagedFiles from codelet-git; outside a git repository both are empty.
  - Moved here from work-unit-scoped-hooks-for-dynamic-validation (supervisor, 2026-11-07): the blocking post-implementing scenario and the two git-context scenarios.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A move from S to T runs the post-S virtual hooks and then the pre-T virtual hooks, all before the move is saved
  #   2. A failing blocking virtual hook blocks the move: the status is unchanged and the error carries a <system-reminder> naming the hook, the work unit and the event
  #   3. A virtual hook with gitContext receives stagedFiles and unstagedFiles in its stdin context; outside a git repository both are empty
  #   4. A virtual hook without gitContext receives no stagedFiles or unstagedFiles keys
  #   5. A move to blocked or to the same status runs no virtual hooks
  #
  # EXAMPLES:
  #   1. Blocking post-implementing hook 'eslint' fails: moving AUTH-001 to validating is blocked and AUTH-001 stays implementing; once eslint passes the move succeeds
  #   2. post-implementing hook 'leave' and pre-validating hook 'enter' both see AUTH-001 still implementing, 'leave' first
  #   3. Staged src/auth.ts and src/utils.ts, unstaged src/hooks/virtual.ts: a gitContext hook reads those lists on stdin
  #   4. Outside a git repository a gitContext hook reads empty lists
  #
  # ========================================
  Background: User Story
    As a AI agent working a work unit with virtual hooks
    I want to have post-<status> virtual hooks run and git-context hooks receive the changed files
    So that quality gates added with add-virtual-hook (including --git-context ones) actually guard the workflow

  Scenario: Blocking virtual hook prevents workflow transition until fixed
    Given work unit "AUTH-001" has a virtual hook "eslint" configured as blocking at post-implementing
    And the work unit is in implementing status
    And the eslint hook fails with "3 errors found" on stderr
    When I run "fspec update-work-unit-status AUTH-001 validating"
    Then the transition should be blocked
    And the error should contain a system-reminder naming "eslint", "AUTH-001" and "post-implementing"
    And the error should contain "3 errors found"
    And work unit status should be "implementing"
    When the eslint hook passes
    And I run "fspec update-work-unit-status AUTH-001 validating" again
    Then the transition should succeed
    And work unit status should be "validating"

  Scenario: Leaving and entering hooks run before the move
    Given work unit "AUTH-001" is in implementing status
    And it has a virtual hook "leave" at post-implementing and a virtual hook "enter" at pre-validating
    And each hook records its name and the saved status of "AUTH-001"
    When I run "fspec update-work-unit-status AUTH-001 validating"
    Then the transition should succeed
    And the hooks should have recorded "leave implementing" and then "enter implementing"

  Scenario: Create virtual hook with git context for changed files only
    Given I am working on work unit "AUTH-001" in a git repository
    And I have staged files: src/auth.ts, src/utils.ts
    And I have unstaged files: src/hooks/virtual.ts
    And work unit "AUTH-001" has a virtual hook "eslint-changed" with git context at post-implementing
    When I run "fspec update-work-unit-status AUTH-001 validating"
    Then the hook context should include git info: {"stagedFiles": ["src/auth.ts", "src/utils.ts"], "unstagedFiles": ["src/hooks/virtual.ts"]}

  Scenario: Detect git staged and unstaged files for hooks
    Given I am not in a git repository
    And work unit "AUTH-001" has a virtual hook with gitContext=true and a virtual hook without it at post-implementing
    When I run "fspec update-work-unit-status AUTH-001 validating"
    Then the gitContext hook context should have empty stagedFiles and unstagedFiles arrays
    And the other hook context should have no stagedFiles or unstagedFiles keys
    And both hooks should receive their context via stdin as JSON
