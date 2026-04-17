@done
@lifecycle-hooks
@agent-core
Feature: Agent Lifecycle Hooks — Extend fspec-hooks.json with Rust Agent Core Events
  """
  Config format is the existing fspec-hooks.json JSON schema: { global?: { timeout, shell }, hooks: Record<string, HookGroup[] | HookDefinition[]> } — agent events use HookGroup[] with optional matcher, while CLI events continue to use HookDefinition[]
  Config paths: user-level ~/.fspec/fspec-hooks.json + project-level spec/fspec-hooks.json; Rust engine reads both, merges agent lifecycle hooks (user-level first, project-level appended); matching the two-level pattern from blocklist (~/.fspec/blocklist.json + .fspec/blocklist.json)
  Pre/post tool use hooks require wrapping rig's auto-execution: recommended approach is a Tool::call() wrapper middleware (or ToolSet::call interception) since rig internally invokes tools during streaming; the wrapper runs pre-hooks before delegating to the real Tool::call and post-hooks after
  Engine threaded through BackgroundSession as Option<LifecycleHookEngine> (Arc-wrapped inner for Clone+Send); notification hooks use a global OnceLock<RwLock<Option<Engine>>> since notifications fire outside the agent loop
  Must maintain Claude Code JSON protocol compatibility for hook script reuse across Claude Code, VTCode, and fspec — same exit code semantics, same JSON response fields (continue, decision, reason, hookSpecificOutput, permissionDecision)
  Module structure in Rust: core engine lives alongside existing tool infrastructure; submodules for config (serde JSON deserialization), compiled (regex compilation), engine (async execution with tokio::process::Command), payloads (per-event JSON payload structs), types (outcome enums), and interpret/ (output parsing per event type)
  Agent lifecycle hook integration points in the agent loop (session_manager.rs): session_start fires after session creation before first prompt; session_end fires on session cleanup/destroy; user_prompt_submit fires after input received before calling run_agent_stream_internal; notification fires via the global engine
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Agent lifecycle hooks extend the existing fspec-hooks.json config format; agent events coexist alongside existing fspec CLI command events (pre-update-work-unit-status, post-implementing, etc.) in the same hooks record
  #   2. Two-level config hierarchy: user-level ~/.fspec/fspec-hooks.json (base) merged with project-level spec/fspec-hooks.json (overrides); project hooks take precedence, matching the existing blocklist/stage-permissions pattern
  #   3. 6 agent lifecycle events supported: session_start, session_end, user_prompt_submit, pre_tool_use, post_tool_use, notification; each is a key in the hooks record containing an array of hook groups
  #   4. Agent lifecycle hook entries use a hook group format: each entry has an optional 'matcher' (regex pattern for tool name filtering on pre/post_tool_use events) and a 'hooks' array of sequential commands; this differs from existing fspec CLI hooks which are flat HookDefinition arrays
  #   5. Hook matchers are regex patterns compiled at engine startup with ^(?:PATTERN)$ anchoring; empty or absent matcher means match-all; invalid regex is a startup error that prevents engine creation
  #   6. Commands execute as sh -c with JSON payload piped to stdin and environment variables: FSPEC_PROJECT_DIR (workspace path), FSPEC_SESSION_ID (session UUID), FSPEC_HOOK_EVENT (event name e.g. PreToolUse), FSPEC_TRANSCRIPT_PATH (session transcript file path)
  #   7. Each command has a configurable timeout (default from global.timeout or 60s); on timeout the child process is killed (SIGKILL after grace period) and the outcome is treated as Deny for pre_tool_use (safety-first) or Warning for other events
  #   8. Hook JSON response format is Claude Code compatible: fields include continue (bool), decision (string), reason (string), suppressOutput (bool), hookSpecificOutput (object with hookEventName, permissionDecision, additionalContext); this enables hook script reuse across Claude Code, VTCode, and fspec
  #   9. pre_tool_use hooks can return four decisions: Allow (auto-approve, skip permission prompt), Deny (block tool call), Ask (force interactive prompt to user), Continue (no opinion, proceed with normal policy)
  #   10. Exit code semantics: exit 0 = success; exit 2 + stderr = deny/block; exit 2 without stderr = warning (continue); timeout = deny for pre_tool_use, warning for others; any other non-zero exit = hook failure (non-blocking warning)
  #   11. Hooks can inject additional context as system messages into the conversation via the additionalContext field in JSON output or via plain text stdout for session_start/user_prompt_submit events
  #   12. pre_tool_use hooks short-circuit on definitive decisions: if any hook group returns Allow or Deny, remaining hook groups for that event are skipped; Continue means no opinion and evaluation continues to next group
  #   13. The lifecycle hook engine is Option<LifecycleHookEngine> — None when no agent lifecycle events are configured in either config file, avoiding any overhead for non-hook users
  #   14. The Rust engine must distinguish agent lifecycle events (session_start, pre_tool_use, etc.) from fspec CLI command events (pre-update-work-unit-status, post-implementing) — only agent lifecycle events are processed by the Rust engine; CLI command events continue to be handled by the existing TypeScript executor
  #   15. JSON payloads piped to hook stdin differ per event type: SessionStart includes session_id, cwd, source (startup/resume); UserPromptSubmit includes prompt text; PreToolUse includes tool_name and tool_input; PostToolUse adds tool_response; all include hook_event_name and transcript_path
  #   16. user_prompt_submit hooks can block user input: if hook returns exit 2 or JSON continue:false, the prompt is rejected and the agent never sees it; the agent loop returns to waiting for input
  #   17. Config is read and compiled once at session creation; hooks are NOT hot-reloaded during a session (unlike the blocklist which hot-reloads every check); a new session picks up config changes
  #   18. The global.timeout and global.shell fields from the existing fspec-hooks.json GlobalConfig are respected by the Rust engine for agent lifecycle hooks
  #   19. Concatenate — user-level hooks run first, then project-level hooks append after; both execute for the same event
  #   20. Mixed format — pre_tool_use/post_tool_use use HookGroup[] with matcher regex; session_start, session_end, user_prompt_submit, notification use the simpler existing HookDefinition[] format (name, command, blocking, timeout)
  #   21. No conditions on agent hooks — agent lifecycle hooks get name, command, blocking, timeout only; condition filtering (tags/prefix/epic/estimate) remains a fspec CLI-only concept
  #
  # EXAMPLES:
  #   1. User adds a pre_tool_use entry in spec/fspec-hooks.json with matcher 'Bash' and a security check script; when the agent tries to execute rm -rf /, the hook exits code 2 with stderr 'Destructive command blocked', and the tool call is denied
  #   2. User adds a session_start hook in ~/.fspec/fspec-hooks.json that outputs JSON with additionalContext: 'Always follow company coding standards'; every new session across all projects injects this as a system message at startup
  #   3. User adds a post_tool_use hook in spec/fspec-hooks.json with matcher 'Write|Edit' that runs a linter; after every write/edit operation, the linter output is injected as additional context so the agent can see and fix lint warnings
  #   4. User adds a user_prompt_submit hook that validates prompts against a company policy; when a forbidden prompt is submitted, the hook exits code 2 with stderr explaining the policy violation; the prompt is rejected and the agent never processes it
  #   5. No agent lifecycle events are configured in either config file; the agent loop runs with zero hook overhead — no engine instantiated, no checks on each tool call or prompt
  #   6. A hook command hangs beyond its configured timeout; the process is killed and the user sees a warning; for pre_tool_use the tool call is denied (safety-first); for post_tool_use the warning is logged but execution continues
  #   7. pre_tool_use hook returns JSON with permissionDecision: 'allow' — the tool executes immediately without any permission prompt to the user, bypassing the normal blocklist/stage-permission checks
  #   8. Multiple pre_tool_use hook groups match the same tool; first group returns Continue (no opinion), second returns Deny — the tool call is denied because evaluation short-circuits on the first definitive decision
  #   9. User has a session_start hook in ~/.fspec/fspec-hooks.json and a different session_start hook in spec/fspec-hooks.json; both execute — the user-level hooks run first, then project-level hooks (project can override or supplement user-level)
  #   10. User has existing fspec CLI hooks (pre-update-work-unit-status, post-implementing) alongside new agent hooks (pre_tool_use, session_start) in the same spec/fspec-hooks.json; the Rust agent engine only processes agent lifecycle events and ignores CLI events, while the TypeScript executor only processes CLI events
  #   11. User configures a session_end hook that posts a summary to Slack; when the agent session ends (completed, cancelled, or error), the hook receives the reason in its JSON payload and sends the notification
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should user-level and project-level hooks merge for the same event? Option A: concatenate (user-level first, then project-level, both execute). Option B: project-level replaces user-level for same event. The blocklist uses 'project rules prepended before system rules' (first-match-wins). For hooks we likely want concatenation since both should run.
  #   A: Concatenate — user-level hooks run first, then project-level hooks append after; both execute for the same event
  #
  #   Q: Should the HookGroup format (with matcher) be used for ALL agent events, or only for pre_tool_use/post_tool_use where matching makes sense? For session_start/session_end/user_prompt_submit/notification there's nothing to match against — should these use the simpler existing HookDefinition[] format?
  #   A: Mixed format — pre_tool_use/post_tool_use use HookGroup[] with matcher regex; session_start, session_end, user_prompt_submit, notification use the simpler existing HookDefinition[] format (name, command, blocking, timeout)
  #
  #   Q: Should the existing TypeScript HookDefinition fields (blocking, condition with tags/prefix/epic/estimate filters) also apply to agent lifecycle hooks, or are those fspec CLI-only concepts? Agent hooks don't have a work unit context by default.
  #   A: No conditions on agent hooks — agent lifecycle hooks get name, command, blocking, timeout only; condition filtering (tags/prefix/epic/estimate) remains a fspec CLI-only concept
  #
  # ========================================
  Background: User Story
    As a fspec user
    I want to configure lifecycle hooks for agent sessions in fspec-hooks.json
    So that I can run custom scripts at key agent lifecycle points (session start/end, prompt submission, before/after tool execution, notifications) to enforce policies, inject context, and integrate with external systems

  # --- Config Loading & Merging ---
  @HOOK-014
  Scenario: Load agent lifecycle hooks from project-level fspec-hooks.json
    Given a project-level spec/fspec-hooks.json with a "session_start" hook entry
    When the lifecycle hook engine initializes for a new session
    Then the engine should load and compile the session_start hook
    And the hook should be available for execution at session start

  @HOOK-014
  Scenario: Load agent lifecycle hooks from user-level fspec-hooks.json
    Given a user-level ~/.fspec/fspec-hooks.json with a "session_start" hook entry
    And no project-level spec/fspec-hooks.json exists
    When the lifecycle hook engine initializes for a new session
    Then the engine should load and compile the session_start hook from the user-level config

  @HOOK-014
  Scenario: Concatenate user-level and project-level hooks for the same event
    Given a user-level ~/.fspec/fspec-hooks.json with a "session_start" hook named "company-policy"
    And a project-level spec/fspec-hooks.json with a "session_start" hook named "project-setup"
    When the lifecycle hook engine initializes for a new session
    Then both hooks should be compiled for the session_start event
    And the user-level "company-policy" hook should execute before the project-level "project-setup" hook

  @HOOK-014
  Scenario: Coexistence of agent lifecycle events and fspec CLI events in same config
    Given a spec/fspec-hooks.json containing both "session_start" and "pre-update-work-unit-status" hooks
    When the lifecycle hook engine initializes for a new session
    Then the engine should load only the "session_start" agent lifecycle event
    And the engine should ignore the "pre-update-work-unit-status" fspec CLI event

  @HOOK-014
  Scenario: No agent lifecycle events configured returns None engine
    Given a spec/fspec-hooks.json with only fspec CLI command hooks and no agent lifecycle events
    And no user-level ~/.fspec/fspec-hooks.json exists
    When the lifecycle hook engine attempts to initialize
    Then the engine should return None
    And zero overhead should be added to the agent loop

  @HOOK-014
  Scenario: Invalid regex matcher prevents engine creation
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "[invalid regex"
    When the lifecycle hook engine attempts to initialize
    Then the engine should return an error indicating the invalid regex pattern
    And no hooks should be compiled

  @HOOK-014
  Scenario: Respect global timeout setting from config
    Given a spec/fspec-hooks.json with global timeout set to 30 seconds
    And a "session_start" hook with no per-hook timeout override
    When the session_start hook executes
    Then the hook should use the 30 second global timeout

  @HOOK-014
  Scenario: Config is compiled once at session creation not hot-reloaded
    Given a spec/fspec-hooks.json with a "session_start" hook
    And the lifecycle hook engine has been initialized for a session
    When the spec/fspec-hooks.json file is modified to add a "user_prompt_submit" hook
    Then the running session should not see the new user_prompt_submit hook
    And only a new session should pick up the config change

  # --- Hook Group Format (pre_tool_use / post_tool_use) ---
  @HOOK-014
  Scenario: pre_tool_use hook group with regex matcher filters by tool name
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "Bash"
    And a hook command that exits with code 0
    When the agent invokes the "Bash" tool
    Then the pre_tool_use hook should execute
    When the agent invokes the "Read" tool
    Then the pre_tool_use hook should not execute

  @HOOK-014
  Scenario: pre_tool_use hook group with empty matcher matches all tools
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with no matcher
    And a hook command that exits with code 0
    When the agent invokes the "Bash" tool
    Then the pre_tool_use hook should execute
    When the agent invokes the "Write" tool
    Then the pre_tool_use hook should also execute

  @HOOK-014
  Scenario: Matcher regex is anchored with full-match semantics
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook group with matcher "Bash"
    When the agent invokes a tool named "BashExtended"
    Then the pre_tool_use hook should not execute because "BashExtended" does not match "^(?:Bash)$"

  # --- HookDefinition Format (session/prompt/notification events) ---
  @HOOK-014
  Scenario: session_start hooks use HookDefinition format
    Given a spec/fspec-hooks.json with a "session_start" entry using HookDefinition format
      """
      {
        "hooks": {
          "session_start": [
            { "name": "setup-env", "command": "./hooks/setup.sh", "timeout": 30 }
          ]
        }
      }
      """
    When the lifecycle hook engine initializes for a new session
    Then the "setup-env" hook should be compiled with a 30 second timeout

  @HOOK-014
  Scenario: user_prompt_submit hooks use HookDefinition format
    Given a spec/fspec-hooks.json with a "user_prompt_submit" entry using HookDefinition format
    When a user submits a prompt
    Then the user_prompt_submit hooks should execute sequentially with JSON payload on stdin

  # --- Command Execution ---
  @HOOK-015
  Scenario: Hook command receives JSON payload on stdin
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that echoes stdin to a file
    When the agent invokes the "Bash" tool with input "ls -la"
    Then the hook should receive a JSON payload on stdin containing:
      | field           | value      |
      | hook_event_name | PreToolUse |
      | tool_name       | Bash       |
    And the tool_input field should contain "ls -la"

  @HOOK-015
  Scenario: Hook command receives environment variables
    Given a spec/fspec-hooks.json with a "session_start" hook that writes env vars to a file
    When the session_start hook executes
    Then the hook process should have FSPEC_PROJECT_DIR set to the workspace path
    And the hook process should have FSPEC_SESSION_ID set to the session UUID
    And the hook process should have FSPEC_HOOK_EVENT set to "SessionStart"
    And the hook process should have FSPEC_TRANSCRIPT_PATH set to the transcript file path

  @HOOK-015
  Scenario: SessionStart payload includes session source
    Given a spec/fspec-hooks.json with a "session_start" hook
    When a new session starts fresh
    Then the hook payload should include source "startup"
    When a session is resumed
    Then the hook payload should include source "resume"

  @HOOK-015
  Scenario: PostToolUse payload includes tool response
    Given a spec/fspec-hooks.json with a "post_tool_use" hook group matching all tools
    When the agent invokes the "Read" tool and it returns file contents
    Then the post_tool_use hook should receive a payload containing tool_name, tool_input, and tool_response

  @HOOK-015
  Scenario: SessionEnd payload includes termination reason
    Given a spec/fspec-hooks.json with a "session_end" hook
    When the session ends because the user cancelled it
    Then the hook payload should include reason "cancelled"

  # --- Timeout Handling ---
  @HOOK-015
  Scenario: Hook command killed on timeout
    Given a spec/fspec-hooks.json with a "session_start" hook with timeout 1 second
    And the hook command sleeps for 10 seconds
    When the session_start hook executes
    Then the hook process should be killed after 1 second
    And the outcome should be a warning

  @HOOK-015
  Scenario: pre_tool_use timeout is treated as Deny for safety
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook with timeout 1 second
    And the hook command sleeps for 10 seconds
    When the agent invokes the "Bash" tool
    Then the hook process should be killed after 1 second
    And the tool call should be denied

  @HOOK-015
  Scenario: post_tool_use timeout is treated as Warning
    Given a spec/fspec-hooks.json with a "post_tool_use" hook with timeout 1 second
    And the hook command sleeps for 10 seconds
    When the agent completes a tool call
    Then the hook process should be killed after 1 second
    And a warning should be emitted but execution should continue

  # --- Exit Code Interpretation ---
  @HOOK-015
  Scenario: Exit code 0 means success
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 0
    When the agent invokes a tool
    Then the hook outcome should be Continue (no opinion)
    And the tool call should proceed with normal policy

  @HOOK-015
  Scenario: Exit code 2 with stderr means deny
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 2 and stderr "Destructive command blocked"
    When the agent tries to execute "rm -rf /"
    Then the tool call should be denied
    And the deny reason should contain "Destructive command blocked"

  @HOOK-015
  Scenario: Exit code 2 without stderr means warning
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 2 and empty stderr
    When the agent invokes a tool
    Then the hook outcome should be a warning
    And the tool call should continue

  @HOOK-015
  Scenario: Non-zero exit code other than 2 is a non-blocking warning
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that exits with code 1
    When the agent invokes a tool
    Then a warning should be emitted about the hook failure
    And the tool call should continue

  # --- JSON Response Interpretation (Claude Code Compatible) ---
  @HOOK-015
  Scenario: pre_tool_use hook returns JSON with permissionDecision allow
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
      """
      { "hookSpecificOutput": { "permissionDecision": "allow" } }
      """
    When the agent invokes a tool
    Then the tool should execute immediately without any permission prompt

  @HOOK-015
  Scenario: pre_tool_use hook returns JSON with permissionDecision deny
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
      """
      { "hookSpecificOutput": { "permissionDecision": "deny" }, "decision": "deny", "reason": "Policy violation" }
      """
    When the agent invokes a tool
    Then the tool call should be denied
    And the deny reason should contain "Policy violation"

  @HOOK-015
  Scenario: pre_tool_use hook returns JSON with permissionDecision ask
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
      """
      { "hookSpecificOutput": { "permissionDecision": "ask" } }
      """
    When the agent invokes a tool
    Then the user should be prompted for interactive permission

  @HOOK-015
  Scenario: pre_tool_use hook returns JSON with continue false
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook that outputs JSON:
      """
      { "continue": false, "reason": "Blocked by policy" }
      """
    When the agent invokes a tool
    Then the tool call should be denied
    And the deny reason should contain "Blocked by policy"

  @HOOK-015
  Scenario: Hook injects additional context as system message
    Given a spec/fspec-hooks.json with a "session_start" hook that outputs JSON:
      """
      { "hookSpecificOutput": { "additionalContext": "Always follow company coding standards" } }
      """
    When the session starts
    Then "Always follow company coding standards" should be injected as a system message into the conversation

  @HOOK-015
  Scenario: session_start hook injects context via plain text stdout
    Given a spec/fspec-hooks.json with a "session_start" hook that outputs plain text "Remember: use TypeScript only"
    When the session starts
    Then "Remember: use TypeScript only" should be injected as additional context

  @HOOK-015
  Scenario: post_tool_use hook injects additional context
    Given a spec/fspec-hooks.json with a "post_tool_use" hook matching "Write|Edit" that outputs JSON:
      """
      { "hookSpecificOutput": { "additionalContext": "Lint warning: unused import on line 5" } }
      """
    When the agent completes a Write tool call
    Then "Lint warning: unused import on line 5" should be injected as a system message

  # --- pre_tool_use Short-Circuit ---
  @HOOK-017
  Scenario: pre_tool_use short-circuits on Allow decision
    Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
      | group  | matcher | decision |
      | first  | Bash    | Allow    |
      | second | Bash    | Deny     |
    When the agent invokes the "Bash" tool
    Then only the first hook group should execute
    And the tool should be allowed (second group's Deny is never reached)

  @HOOK-017
  Scenario: pre_tool_use short-circuits on Deny decision
    Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
      | group  | matcher | decision |
      | first  | Bash    | Deny     |
      | second | Bash    | Allow    |
    When the agent invokes the "Bash" tool
    Then only the first hook group should execute
    And the tool call should be denied (second group's Allow is never reached)

  @HOOK-017
  Scenario: pre_tool_use Continue does not short-circuit
    Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
      | group  | matcher | decision |
      | first  | Bash    | Continue |
      | second | Bash    | Deny     |
    When the agent invokes the "Bash" tool
    Then both hook groups should execute
    And the tool call should be denied by the second group

  @HOOK-017
  Scenario: pre_tool_use all groups return Continue falls through
    Given a spec/fspec-hooks.json with two "pre_tool_use" hook groups:
      | group  | matcher | decision |
      | first  | Bash    | Continue |
      | second | Bash    | Continue |
    When the agent invokes the "Bash" tool
    Then both hook groups should execute
    And the final decision should be Continue (fall through to normal permission checks)

  # --- user_prompt_submit Blocking ---
  @HOOK-016
  Scenario: user_prompt_submit hook blocks forbidden prompt
    Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that exits code 2 with stderr "Policy violation: prompt contains forbidden content"
    When the user submits a prompt
    Then the prompt should be rejected
    And the agent should never see the prompt
    And the agent loop should return to waiting for input

  @HOOK-016
  Scenario: user_prompt_submit hook blocks via JSON continue false
    Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that outputs JSON:
      """
      { "continue": false, "reason": "Prompt violates content policy" }
      """
    When the user submits a prompt
    Then the prompt should be rejected
    And the block reason should contain "Prompt violates content policy"

  @HOOK-016
  Scenario: user_prompt_submit hook allows and injects context
    Given a spec/fspec-hooks.json with a "user_prompt_submit" hook that outputs JSON:
      """
      { "continue": true, "hookSpecificOutput": { "additionalContext": "User is an admin" } }
      """
    When the user submits a prompt
    Then the prompt should be allowed through to the agent
    And "User is an admin" should be injected as additional context

  # --- session_end ---
  @HOOK-016
  Scenario: session_end hook receives termination reason and executes
    Given a spec/fspec-hooks.json with a "session_end" hook named "notify-slack"
    When the agent session ends with reason "completed"
    Then the "notify-slack" hook should execute
    And the hook payload should include session_id, cwd, reason "completed", and transcript_path

  # --- notification ---
  @HOOK-016
  Scenario: notification hook fires via global engine
    Given a spec/fspec-hooks.json with a "notification" hook
    When a notification event fires with type "permission_prompt" and title "Tool Permission"
    Then the notification hook should execute
    And the hook payload should include notification_type, title, and message

  # --- Sequential Execution Within Hook Groups ---
  @HOOK-016
  Scenario: Multiple commands in a hook group execute sequentially
    Given a spec/fspec-hooks.json with a "pre_tool_use" hook group containing two commands
    When the agent invokes a tool matching the group
    Then the first command should complete before the second command starts
    And both command results should contribute to the hook group outcome

  @HOOK-017
  Scenario: pre_tool_use hook fires for BashTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the BashTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the command should never have executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for ReadTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the ReadTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the file should never have been read

  @HOOK-017
  Scenario: pre_tool_use hook fires for WriteTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the WriteTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the file should never have been written

  @HOOK-017
  Scenario: pre_tool_use hook fires for EditTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the EditTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the file should never have been modified

  @HOOK-017
  Scenario: pre_tool_use hook fires for LsTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the LsTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the directory should never have been listed

  @HOOK-017
  Scenario: pre_tool_use hook fires for GlobTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the GlobTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the glob should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for GrepTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the GrepTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the search should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for AstGrepTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the AstGrepTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the AST search should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for AstGrepRefactorTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the AstGrepRefactorTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the refactor should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for FspecTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the FspecTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the fspec command should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for BridgeTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the BridgeTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the bridge action should never have been executed

  @HOOK-017
  Scenario: pre_tool_use hook fires for ApplyPatchTool
    Given a registered pre_tool_use Deny hook handler for the session
    When the ApplyPatchTool.call() method is invoked
    Then the tool should return ToolError::Blocked with the deny reason
    Then the patch should never have been applied

  @HOOK-017
  Scenario: pre_tool_use Allow handler lets native tool proceed
    Given a registered pre_tool_use Allow hook handler for the session
    When the BashTool.call() method is invoked with a safe command
    Then the command should execute successfully
    Then the output should contain the command result

  @HOOK-017
  Scenario: No registered handler lets native tool proceed without overhead
    Given no pre_tool_use hook handler is registered for the session
    When the BashTool.call() method is invoked
    Then the command should execute successfully with no hook overhead
