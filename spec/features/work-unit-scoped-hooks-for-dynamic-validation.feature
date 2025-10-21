@done
@cli
@workflow-automation
@hooks
@phase2
@HOOK-011
Feature: Work unit-scoped hooks for dynamic validation
  """
  Virtual hooks extend the existing fspec hooks system (src/hooks/) to support work unit-scoped ephemeral hooks. Key components: virtualHooks array in WorkUnit type, commands (add-virtual-hook, list-virtual-hooks, remove-virtual-hook, copy-virtual-hooks), hook executor integration to run virtual hooks BEFORE global hooks, git context provider for staged/unstaged file lists. Implementation leverages existing hook infrastructure (executor, discovery, conditions, formatting) with new virtual hook discovery layer.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Ephemeral by default - hooks run once when requested without persisting. AI can offer to convert useful virtual hooks into permanent hooks (saved to spec/fspec-hooks.json) if the user wants to reuse them.
  #   2. Virtual hooks are created conversationally - AI proactively asks user 'Do you want me to run anything at specific stages?' User names tools (e.g., 'eslint', 'stylelint'). AI then: 1) Checks if tools exist/are installed, 2) Looks up how to use them (help commands, web search), 3) Creates virtual hooks dynamically. No predefined registry needed.
  #   3. When AI asks 'Do you want me to run anything at specific stages?', it MUST list the available hook events (post-implementing, post-testing, post-validating, pre-specifying, etc.) so the human knows their options
  #   4. At the end of specifying phase - after specifications are complete and before moving to testing. This is when AI has full context of what will be built and can meaningfully ask about quality checks.
  #   5. AI must phrase the question clearly with examples: 'Do you want me to run a command or program at any stage of this story? For example, I could run eslint after implementing, or run prettier before validating. Here are the available stages: [list stages]'
  #   6. Scoped to current work unit only by default. After the virtual hook executes successfully, AI should ask: 'This [tool] hook worked well. Do you want to make it permanent so it runs for all work units at this stage?' If yes, AI saves it to spec/fspec-hooks.json using 'fspec add-hook' command.
  #   7. AI asks user when setting up hook: 'Should I block transitions if [tool] fails?' If user says blocking: STRICT ENFORCEMENT with <system-reminder> wrapping the failure output, AI CANNOT proceed with workflow transition until issues are fixed. If non-blocking: show output but allow transition.
  #   8. When work unit reaches 'done' status, AI asks user: 'Do you want to keep these virtual hooks for future edits to this story, or remove them?' If keep: hooks remain attached to work unit. If remove: hooks are cleaned up.
  #   9. Virtual hooks stored in work-units.json as a 'virtualHooks' array field on the work unit. This ties hooks to the story lifecycle, persists across sessions, visible in 'fspec show-work-unit', and can be cleaned up when story is done.
  #   10. AI decides based on complexity: simple commands use direct execution (e.g., 'eslint src/'), complex ones generate script files in spec/hooks/.virtual/. If hook fails to execute (command not found, script errors, permission issues), AI must: 1) Detect execution failure, 2) Show error to user with <system-reminder>, 3) Offer to fix or remove the broken hook, 4) Ask user if they want to try a different approach.
  #   11. Unlimited virtual hooks allowed. User can add multiple hooks to different stages (eslint at post-implementing, prettier at post-validating) AND multiple hooks to the same stage (eslint AND stylelint both at post-implementing). Hooks execute sequentially in the order they were added.
  #   12. Both dedicated commands AND AI conversational management. Commands: 'fspec add-virtual-hook HOOK-011 <event> <command>', 'fspec list-virtual-hooks HOOK-011', 'fspec remove-virtual-hook HOOK-011 <hook-name>'. Virtual hooks also appear in 'fspec show-work-unit HOOK-011' output. AI uses these commands when user says 'add eslint hook' or 'remove the prettier hook'. Commands MUST have comprehensive --help documentation so AI can use them correctly.
  #   13. Virtual hooks run BEFORE global hooks. Execution order: 1) Virtual hooks (work unit-specific), 2) Global hooks (from spec/fspec-hooks.json). This allows work unit-specific validation to happen first before broader project-wide checks.
  #   14. Both dedicated command AND AI conversational management. Command: 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011' (copies all hooks) or 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011 --hook-name eslint' (copies specific hook). AI can handle conversationally: 'use the same hooks as AUTH-001' (all) or 'copy the eslint hook from AUTH-001' (specific). Command must have comprehensive --help.
  #   15. Virtual hooks can receive git context (staged/unstaged files) via hook context JSON. AI can create hooks that target specific changed files. For complex hooks needing git context, AI generates script files in spec/hooks/.virtual/ that process the file list.
  #   16. Git context is OPTIONAL. When setting up a virtual hook, AI asks: 'Do you want this hook to target only changed files (staged/unstaged)? For example, I can run eslint only on your modified .ts files.' If yes: AI generates script that uses git context. If no: command runs as-is without git context (e.g., 'eslint src/' runs against entire codebase).
  #
  # EXAMPLES:
  #   1. User is working on AUTH-001. At end of specifying, AI asks: 'Do you want me to run a command or program at any stage? For example, eslint after implementing. Available stages: post-implementing, post-testing, post-validating.' User says: 'Yes, run eslint and prettier.' AI creates two virtual hooks, asks if blocking, stores in work-units.json.
  #   2. Virtual hook 'eslint' runs at post-implementing. Eslint finds 3 errors. Hook is blocking. AI shows errors wrapped in <system-reminder>, CANNOT proceed to validating until user fixes errors. User fixes errors, re-runs transition, eslint passes, transition succeeds.
  #   3. Virtual hook runs successfully. AI asks: 'The eslint hook worked well. Do you want to make it permanent for all work units?' User says yes. AI runs 'fspec add-hook post-implementing eslint spec/hooks/.virtual/eslint-HOOK-011.sh --blocking' to save to spec/fspec-hooks.json.
  #   4. Work unit reaches done status. AI asks: 'Do you want to keep these virtual hooks for future edits to this story, or remove them?' User says remove. AI cleans up virtualHooks array from work-units.json and deletes generated script files.
  #   5. User says 'use the same hooks as AUTH-001'. AI runs 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011', copies all virtual hooks (eslint, prettier) with their configurations (blocking status, event names) to current work unit.
  #   6. Virtual hook command 'eslint src/' fails to execute (command not found). AI detects execution failure, shows error in <system-reminder>: 'Hook failed to execute: eslint: command not found. Install eslint or check PATH.' AI offers to remove broken hook or try different command.
  #   7. User runs 'fspec show-work-unit HOOK-011'. Output includes 'Virtual Hooks' section showing: 'post-implementing: eslint (blocking), prettier (non-blocking)', 'post-validating: test-coverage (blocking)'. User can see all active hooks at a glance.
  #   8. User says 'run eslint only on changed files'. AI creates virtual hook with script in spec/hooks/.virtual/eslint-changed-files.sh. Hook context includes git info: {stagedFiles: ['src/auth.ts'], unstagedFiles: ['src/utils.ts']}. Script runs: eslint $STAGED_FILES. This is more efficient than linting entire codebase.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should work unit-scoped hooks be saved permanently (persisted in work-units.json) or ephemeral (exist only during the current session)?
  #   A: true
  #
  #   Q: How should the AI discover what virtual hooks are available? Should we have predefined templates (like 'eslint', 'prettier', 'test-coverage') or allow arbitrary commands (like 'npm run custom-lint'), or both?
  #   A: true
  #
  #   Q: When should the AI ask 'Do you want me to run anything at specific stages?' - at the start of the story, at specific workflow stages (like before implementing), or any time during the story?
  #   A: true
  #
  #   Q: When a virtual hook executes (e.g., eslint at post-validating), should it only run for THIS work unit or apply globally to all work units until removed?
  #   A: true
  #
  #   Q: If a virtual hook fails (e.g., eslint finds errors), should it be blocking (prevent workflow transition) or non-blocking (show output but allow transition)? Or should AI ask the user which behavior they want?
  #   A: true
  #
  #   Q: What happens to virtual hooks when the work unit reaches 'done' status? Should they automatically be removed, or persist for potential future edits to this work unit?
  #   A: true
  #
  #   Q: Where should virtual hooks be stored temporarily? Should they be in work-units.json (in a 'virtualHooks' field), or in a separate temporary file, or in memory only during the AI session?
  #   A: true
  #
  #   Q: When AI creates a virtual hook to run a tool (like eslint), should it: 1) Execute the command directly (e.g., store 'eslint src/' as the command), or 2) Generate a temporary script file in spec/hooks/.virtual/ that wraps the command?
  #   A: true
  #
  #   Q: Can a user add multiple virtual hooks to different stages (e.g., eslint at post-implementing AND prettier at post-validating)? And can they add multiple hooks to the SAME stage (e.g., both eslint and stylelint at post-implementing)?
  #   A: true
  #
  #   Q: How should users view and manage their virtual hooks after they're created? Should there be commands like 'fspec list-virtual-hooks HOOK-011' or should they appear in 'fspec show-work-unit HOOK-011' output? And how should users remove unwanted virtual hooks?
  #   A: true
  #
  #   Q: If a work unit has a virtual hook AND there's a global hook configured for the same event (both at post-implementing), what order should they run? Virtual first then global, or global first then virtual?
  #   A: true
  #
  #   Q: How should users copy virtual hooks from one work unit to another? Should there be a command like 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011', or should AI handle it conversationally ('use the same hooks as AUTH-001')? And should it copy all hooks or allow selecting specific ones?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent working on a specific work unit
    I want to attach validation hooks (like eslint) to this work unit dynamically
    So that I can enforce quality checks for this story without modifying global hook configuration

  Scenario: AI prompts for virtual hooks at end of specifying phase
    Given I am working on work unit "AUTH-001" in specifying status
    And I have completed the Example Mapping and generated scenarios
    When I prepare to move to testing status
    Then AI should ask "Do you want me to run a command or program at any stage of this story?"
    And AI should provide examples "For example, I could run eslint after implementing, or run prettier before validating"
    And AI should list available stages "post-implementing, post-testing, post-validating, pre-specifying"
    When user responds "Yes, run eslint and prettier"
    Then AI should ask "At which stage should eslint run?"
    And AI should ask "Should I block transitions if eslint fails?"
    And AI should ask "At which stage should prettier run?"
    And AI should ask "Should I block transitions if prettier fails?"
    And AI should run "fspec add-virtual-hook AUTH-001 <event> <command>" for each hook
    And virtual hooks should be stored in work-units.json under AUTH-001.virtualHooks array

  Scenario: Blocking virtual hook prevents workflow transition until fixed
    Given work unit "AUTH-001" has a virtual hook "eslint" configured as blocking at post-implementing
    And the work unit is in implementing status
    When I run "fspec update-work-unit-status AUTH-001 validating"
    And the eslint hook executes and finds 3 errors
    Then the transition should be blocked
    And AI should display errors wrapped in <system-reminder> tags
    And AI should show "✗ Blocked by virtual hook 'eslint'"
    And AI CANNOT proceed with transition until errors are fixed
    When user fixes the 3 eslint errors
    And I run "fspec update-work-unit-status AUTH-001 validating" again
    And the eslint hook executes and passes
    Then the transition should succeed
    And work unit status should be "validating"

  Scenario: Convert successful virtual hook to permanent global hook
    Given work unit "AUTH-001" has a virtual hook "eslint" that ran successfully
    When the hook completes without errors
    Then AI should ask "The eslint hook worked well. Do you want to make it permanent so it runs for all work units at this stage?"
    When user responds "yes"
    Then AI should run "fspec add-hook post-implementing eslint spec/hooks/.virtual/eslint-AUTH-001.sh --blocking"
    And the hook should be saved to spec/fspec-hooks.json
    And the hook should run for all future work units at post-implementing stage

  Scenario: Clean up virtual hooks when work unit reaches done status
    Given work unit "AUTH-001" has virtual hooks configured
    And the work unit reaches "done" status
    When the status change completes
    Then AI should ask "Do you want to keep these virtual hooks for future edits to this story, or remove them?"
    When user responds "remove"
    Then AI should clear the virtualHooks array from work-units.json for AUTH-001
    And AI should delete any generated script files in spec/hooks/.virtual/

  Scenario: Copy virtual hooks from one work unit to another
    Given work unit "AUTH-001" has virtual hooks: eslint (blocking, post-implementing), prettier (non-blocking, post-validating)
    And I am working on work unit "HOOK-011"
    When user says "use the same hooks as AUTH-001"
    Then AI should run "fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011"
    And HOOK-011 should have the same virtual hooks as AUTH-001
    And hook configurations (blocking status, event names, commands) should be identical

  Scenario: Handle virtual hook execution failure gracefully
    Given work unit "AUTH-001" has virtual hook "eslint" with command "eslint src/"
    When the hook attempts to execute
    And the command fails with "eslint: command not found"
    Then AI should detect the execution failure
    And AI should display error in <system-reminder>: "Hook failed to execute: eslint: command not found. Install eslint or check PATH."
    And AI should offer "Do you want to remove this broken hook or try a different command?"
    When user chooses to remove
    Then AI should run "fspec remove-virtual-hook AUTH-001 eslint"

  Scenario: Display virtual hooks in work unit details
    Given work unit "HOOK-011" has the following virtual hooks:
      | Event             | Hook          | Blocking |
      | post-implementing | eslint        | yes      |
      | post-implementing | prettier      | no       |
      | post-validating   | test-coverage | yes      |
    When I run "fspec show-work-unit HOOK-011"
    Then the output should include a "Virtual Hooks" section
    And it should show "post-implementing: eslint (blocking), prettier (non-blocking)"
    And it should show "post-validating: test-coverage (blocking)"

  Scenario: Create virtual hook with git context for changed files only
    Given I am working on work unit "AUTH-001"
    And I have staged files: src/auth.ts, src/utils.ts
    And I have unstaged files: src/hooks/virtual.ts
    When user says "run eslint only on changed files"
    Then AI should ask "Do you want this hook to target only changed files? For example, I can run eslint only on your modified .ts files."
    When user responds "yes"
    Then AI should generate a script file "spec/hooks/.virtual/eslint-changed-files-AUTH-001.sh"
    And the hook context should include git info: {"stagedFiles": ["src/auth.ts", "src/utils.ts"], "unstagedFiles": ["src/hooks/virtual.ts"]}
    And the script should run "eslint $STAGED_FILES"
    When the hook executes
    Then it should only lint the changed files, not the entire codebase

  Scenario: Detect shell commands vs script paths correctly
    Given I have a hook command './script.sh' with explicit path prefix
    When the command is evaluated by isShellCommand utility
    Then it should be classified as a script path, not a shell command
    And the validator should check if the file exists
    When I have a hook command 'npm run lint' without path prefix
    Then it should be classified as a shell command
    And the validator should not check file existence
    When I have a script 'my-hook.sh' that exists in the project root
    Then it should be classified as a script path (file exists)
    When I have a command 'nonexistent-script.sh' that does not exist
    Then it should be classified as a shell command (file not found)

  Scenario: Generate executable script files for complex virtual hooks
    Given I create a virtual hook with git context enabled
    When I run "fspec add-virtual-hook AUTH-001 --event post-implementing --command 'eslint' --git-context"
    Then a script file should be created at "spec/hooks/.virtual/AUTH-001-[hook-name].sh"
    And the script should be executable (mode 0o755)
    And the script should read hook context JSON from stdin
    And the script should extract stagedFiles and unstagedFiles from context
    And the script should pass file lists to the command
    When the hook is removed
    Then the script file should be deleted automatically

  Scenario: Detect git staged and unstaged files for hooks
    Given I am in a git repository
    And I have staged files: src/auth.ts, src/utils.ts
    And I have unstaged modified files: src/hooks/executor.ts
    When a virtual hook with gitContext=true executes
    Then the hook context should include stagedFiles: ["src/auth.ts", "src/utils.ts"]
    And the hook context should include unstagedFiles: ["src/hooks/executor.ts"]
    And the hook should receive this context via stdin as JSON
    When I am not in a git repository
    Then the hook context should have empty stagedFiles and unstagedFiles arrays

  Scenario: System reminder prompts AI about virtual hooks at phase transitions
    Given I am working on work unit "AUTH-001" in specifying status
    When I run "fspec update-work-unit-status AUTH-001 testing"
    Then a system reminder should be emitted to AI
    And the reminder should mention "VIRTUAL HOOKS: Consider quality checks"
    And the reminder should list available hook events
    And the reminder should provide command examples
    When I transition work unit "AUTH-001" to done status with 2 virtual hooks
    Then a system reminder should ask about cleanup
    And the reminder should present keep vs remove options
    And the reminder should ask AI to prompt the user for decision
