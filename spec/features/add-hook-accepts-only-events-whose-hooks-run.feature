@done
@validation
@BUG-590
@hooks
@cli
Feature: add-hook accepts only events whose hooks run
  """
  Architecture notes:
  - rust/fspec-core/src/command_hooks/events.rs: accepted_event(event) is the one check: pre-/post-<command> where is_hooked_command(command) (HOOK-018), or pre-/post-<status> of a WorkUnitStatus (HOOK-020). add-hook (commands/add_hook.rs) calls it before taking the spec lock or reading spec/fspec-hooks.json.
  - The CLI-016 guidance contract (rust/fspec/src/guidance_contract_tests.rs) checks every 'fspec add-hook <event>' mention with the same accepted_event, so guidance and the command agree by construction.
  - add-hook's help config (help/configs/add_hook.rs) and its rendered fixture (rust/fspec/tests/fixtures/help/add-hook.txt) name the accepted event forms and point to add-virtual-hook for one work unit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The CLI-016 guidance contract also checks that every 'fspec add-hook <event>' mention in guidance and help names an event add-hook accepts, failing with the source and line otherwise
  #   2. A global hook fires on pre-<command> or post-<command> of an fspec command that runs command hooks (HOOK-018 is_hooked_command: canonical commands except add-hook, remove-hook, list-hooks and validate-hooks), or on pre-<status> or post-<status> of a workflow status (backlog, specifying, testing, implementing, validating, done, blocked), run by update-work-unit-status for every work unit (HOOK-020; product owner, 2026-11-07)
  #   3. add-hook rejects any other event before writing spec/fspec-hooks.json: the error names the event and says global hooks run on pre-<command>/post-<command> of an fspec command or pre-<status>/post-<status> of a workflow status
  #   4. add-hook's help (arguments, examples, common patterns, notes) uses only accepted events and says a status hook runs for every work unit, pointing to add-virtual-hook for a hook on one work unit
  #
  # EXAMPLES:
  #   1. fspec add-hook pre-update-work-unit-status check-ready --command spec/hooks/check.sh adds the hook
  #   2. fspec add-hook post-add-hook audit --command x.sh is rejected: add-hook runs no command hooks
  #   3. fspec add-hook pre-implementing lint --command spec/hooks/lint.sh adds a global status hook that update-work-unit-status runs for every work unit entering implementing
  #   4. fspec add-hook pre-lint check --command x.sh is rejected naming pre-lint; spec/fspec-hooks.json is unchanged
  #   5. An init doc mentioning 'fspec add-hook pre-lint check' fails the guidance contract naming the doc and line
  #   6. fspec add-hook pre-lint check --command x.sh fails with exactly (the message alone, no args hint): unknown hook event 'pre-lint': a global hook runs on pre-<command> or post-<command> of an fspec command that runs command hooks, or on pre-<status> or post-<status> of a workflow status (backlog, specifying, testing, implementing, validating, done, blocked)
  #
  # QUESTIONS (ANSWERED):
  #   Q: should add-hook reject a non-command event or only warn about it? Conservative choice taken: reject (no hook that can never fire is written), since warning would still store a dead hook.
  #   A: Provisional, pending the product owner (helper, 2026-11-07): reject before writing; the error names the event and, for a status event, add-virtual-hook. Existing configs are not touched (validate-hooks is unchanged).
  #
  #   Q: work-unit-scoped-hooks-for-dynamic-validation has the AI promote a working virtual hook to a permanent one with 'fspec add-hook post-implementing eslint ...' (rule 6, scenario at line 106). Status events never fire as global hooks, so that promotion already writes a dead hook, and with BUG-590 add-hook rejects it. Should global status hooks exist (pre-/post-<status> for every work unit, run by update-work-unit-status), or should the promotion flow change (e.g. to fspec copy-virtual-hooks or a pre-update-work-unit-status hook)?
  #   A: Product owner, 2026-11-07 (via hitl): global status hooks exist (HOOK-020): add-hook accepts pre-/post-<command> and pre-/post-<status>, update-work-unit-status runs status hooks for every work unit, and add-hook rejects anything else before writing, with an exact error. The promotion flow in work-unit-scoped-hooks-for-dynamic-validation stays as written.
  #
  # ASSUMPTIONS:
  #   1. Provisional, pending the product owner (helper, 2026-11-07): reject before writing; the error names the event and, for a status event, add-virtual-hook. Existing configs are not touched (validate-hooks is unchanged).
  #   2. Superseded by the product-owner answer (2026-11-07): status events are accepted (HOOK-020); assumption 1's add-virtual-hook hint for status events no longer applies
  #   3. Provisional (helper, 2026-11-07), for the product owner to confirm: the agent lifecycle events that can live in spec/fspec-hooks.json (session_start, session_end, user_prompt_submit, notification, pre_tool_use, post_tool_use; codelet-core lifecycle_hooks) are rejected by add-hook too, per the decision's 'rejects anything else'; add-hook never documented them, and the tool events need a matcher group add-hook cannot write. pre-blocked is accepted as a status event although HOOK-020's provisional assumption means a move to blocked runs no hooks
  #
  # ========================================
  Background: User Story
    As a developer adding a global hook with add-hook
    I want to be told at once when the event I name can never fire, and see help that only suggests events that do
    So that I do not register a quality gate that silently never runs

  Scenario: A hook on an fspec command event is added
    Given a project without spec/fspec-hooks.json
    When I run "fspec add-hook pre-update-work-unit-status check-ready --command spec/hooks/check.sh"
    Then spec/fspec-hooks.json holds the hook "check-ready" under the event "pre-update-work-unit-status"

  Scenario: A hook on a workflow status event is added as a global status hook
    Given a project without spec/fspec-hooks.json
    When I run "fspec add-hook pre-implementing lint --command spec/hooks/lint.sh"
    Then spec/fspec-hooks.json holds the hook "lint" under the event "pre-implementing"

  Scenario: An event that is neither a command nor a status is rejected before writing
    Given spec/fspec-hooks.json holds one hook under "pre-implementing"
    When I run "fspec add-hook pre-lint check --command x.sh"
    Then it fails with "unknown hook event 'pre-lint': a global hook runs on pre-<command> or post-<command> of an fspec command that runs command hooks, or on pre-<status> or post-<status> of a workflow status (backlog, specifying, testing, implementing, validating, done, blocked)"
    And spec/fspec-hooks.json is byte-for-byte unchanged

  Scenario: An event of a command that runs no command hooks is rejected
    Given a project without spec/fspec-hooks.json
    When I run "fspec add-hook post-add-hook audit --command x.sh"
    Then it fails with the unknown hook event error naming "post-add-hook"
    And no spec/fspec-hooks.json is written

  Scenario: add-hook's help names only accepted events and points to add-virtual-hook for one work unit
    Given add-hook's help config
    When every event in its examples and common patterns is checked
    Then every event is accepted by add-hook
    And its notes say a status hook runs for every work unit and a hook on one work unit is added with add-virtual-hook

  Scenario: A guidance mention of add-hook with a rejected event fails the guidance contract
    Given a source line "fspec add-hook pre-lint check --command x.sh" at line 3 of "sample.md"
    When the guidance mentions are checked
    Then the mention fails naming "sample.md:3" and "unknown hook event 'pre-lint'"
    And every add-hook mention in the real guidance sources and help configs names an accepted event
