@done
@HOOK-020
@hooks
@workflow-automation
@cli
Feature: Global status hooks run for every work unit
  """
  Architecture notes:
  - Global hooks live in spec/fspec-hooks.json (HOOK-002 schema, loaded by rust/fspec-core/src/command_hooks/config.rs). Before HOOK-020 only pre-/post-<command> events ran (with_command_hooks in phases.rs); a pre-/post-<status> entry was loaded but never run.
  - Status moves run in update_work_unit_status.rs GATE 8 through command_hooks::run_status_hooks (virtual_hooks.rs): for S -> T, post-S global hooks, post-S virtual hooks, pre-T global hooks, pre-T virtual hooks, all before the move is saved. A move to the same status or to blocked runs none (HOOK-019 rule 5).
  - Global status hooks use the command-hook executor with the hook's own timeout (or global.timeout, or 60 s) and condition (hook_matches on the moving work unit). Every status hook (global or virtual) gets HookContext::status(id, event, from, to): workUnitId, event, timestamp, fromStatus, toStatus; HOOK-021 gives command hooks the same fields plus their arguments.
  - The command hooks of update-work-unit-status itself (pre-/post-update-work-unit-status) still wrap the whole command, outside the status hooks.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A global hook in spec/fspec-hooks.json whose event is pre-<status> or post-<status> for a workflow status (backlog, specifying, testing, implementing, validating, done, blocked) is a global status hook; add-hook writes it like any other event (product owner, 2026-11-07)
  #   2. update-work-unit-status moving a work unit from S to T runs, before the move is saved, the post-S global hooks, then the work unit's post-S virtual hooks, then the pre-T global hooks, then its pre-T virtual hooks, each group in its configured order; a move to the same status or to blocked runs no status hooks (HOOK-019 rule 5)
  #   3. A global status hook gets the same JSON context on stdin as a virtual status hook ({"workUnitId":"<id>","event":"<event>","timestamp":...}), runs in the project root with its configured timeout (or global.timeout, or 60 s), and runs only when its condition (tags, prefix, epic, estimate) matches the moving work unit
  #   4. A failing blocking global status hook, pre or post, stops the remaining status hooks and blocks the move: the work unit keeps its status and the error is a system-reminder 'BLOCKING HOOK FAILURE: global hook '<name>' for <id> failed at <event>.' with the exit code and the hook's output; a failing non-blocking hook's output is shown and the move proceeds
  #   5. A hooks file that cannot be read or parsed, or a status hook whose script does not exist, is an error that blocks the move; it is never treated as no hooks
  #
  # EXAMPLES:
  #   1. spec/fspec-hooks.json has post-implementing hook 'lint' (blocking, exits 1): moving AUTH-001 from implementing to validating fails with "BLOCKING HOOK FAILURE: global hook 'lint' for AUTH-001 failed at post-implementing." and AUTH-001 stays implementing
  #   2. Global pre-validating hook 'g' and AUTH-001's virtual post-implementing hook 'v' and global post-implementing hook 'p' all append their name to a file: moving AUTH-001 implementing -> validating writes p, v, g in that order
  #   3. A global pre-testing hook that saves its stdin receives {"workUnitId":"AUTH-001","event":"pre-testing",...} when AUTH-001 moves specifying -> testing; a hook with condition prefix [BUG] does not run for AUTH-001
  #   4. Moving AUTH-001 to blocked runs no global status hooks, even a pre-blocked one
  #
  # ASSUMPTIONS:
  #   1. Provisional (helper, 2026-11-07): global status hooks follow the virtual status-hook contract exactly, so a move to blocked runs none and a pre-blocked hook never runs (add-hook still accepts it per the product-owner decision); global hooks of an event run before the work unit's virtual hooks of the same event
  #
  # ========================================
  Background: User Story
    As a team using fspec's ACDD workflow
    I want to register a hook once for a workflow status (e.g. post-implementing) that runs for every work unit
    So that a quality gate applies to all work units, and a virtual hook promoted with add-hook keeps running instead of becoming a dead hook

  Scenario: A failing blocking global post-status hook blocks the move
    Given spec/fspec-hooks.json has a blocking post-implementing hook "lint" that exits 1
    And work unit "AUTH-001" is implementing
    When I run "fspec update-work-unit-status AUTH-001 validating"
    Then the command fails with "BLOCKING HOOK FAILURE: global hook 'lint' for AUTH-001 failed at post-implementing."
    And work unit "AUTH-001" is still implementing

  Scenario: Global and virtual status hooks run in event order
    Given a global post-implementing hook "p" and a global pre-validating hook "g" that append their name to a file
    And work unit "AUTH-001" has a virtual post-implementing hook "v" that appends its name to the file
    When work unit "AUTH-001" moves from implementing to validating
    Then the file lists "p", "v" and "g" in that order
    And work unit "AUTH-001" is validating

  Scenario: A global status hook receives the work unit context and honours its condition
    Given a global pre-validating hook that saves its stdin
    And a global pre-validating hook with condition prefix "BUG" that writes a marker file
    When work unit "AUTH-001" moves from implementing to validating
    Then the saved stdin has workUnitId "AUTH-001", event "pre-validating", fromStatus "implementing" and toStatus "validating"
    And the marker file does not exist

  Scenario: A move to blocked runs no global status hooks
    Given a global post-implementing hook and a global pre-blocked hook that write marker files
    When work unit "AUTH-001" moves from implementing to blocked
    Then neither marker file exists

  Scenario: An unparseable hooks file blocks a status move
    Given spec/fspec-hooks.json is not valid JSON
    When work unit "AUTH-001" moves from implementing to validating
    Then the command fails naming "Invalid JSON in spec/fspec-hooks.json"
    And work unit "AUTH-001" is still implementing
