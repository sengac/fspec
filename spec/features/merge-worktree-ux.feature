@git-integration
@tui
@GIT-037
Feature: Merge worktree UX: confirmation summary with press-enter-to-close and conflict guidance

  """
  Generic 'action prompt' mechanism threaded through the component stack:

1. ActionPrompt interface:
   interface ActionPrompt { message: string; onConfirm: () => void | Promise<void> }
   Note: onConfirm may be async (e.g. destroySession). The handler must await it.

2. AgentView state:
   const [actionPrompt, setActionPrompt] = useState<ActionPrompt | null>(null)
   Passes actionPrompt + setActionPrompt to both InputTransition and the merge handler context.

3. InputTransition new prop:
   actionPrompt?: ActionPrompt | null
   Also receives: clearActionPrompt: () => void (bound to setActionPrompt(null) in AgentView)

4. InputTransition rendering (when actionPrompt is set):
   <Text color='green'>✓ {actionPrompt.message}</Text> <Text dimColor>(Press Enter or Esc to close)</Text>

5. InputTransition keyboard handler (useInputCompat, priority MEDIUM):
   Enter or Escape → await actionPrompt.onConfirm(), then clearActionPrompt()
   All other keys → consumed (blocked)
   The auto-clear ensures the prompt is always cleaned up, whether onConfirm unmounts the component or not. Future callers that don't unmount get correct behavior for free.

6. MultiLineInput does NOT need changes — InputTransition short-circuits rendering before reaching MultiLineInput when actionPrompt is active (same pattern as isPaused).

7. Rendering priority (early return order in InputTransition):
   isPaused && pauseInfo → existing pause rendering
   actionPrompt → NEW action prompt rendering
   animationPhase === 'loading' → existing loading/compaction
   ... rest of existing animation phases
   isPaused takes precedence over actionPrompt. In practice they should never overlap (no tool calls during slash commands), but the ordering is safe if they do.
  mergeWorktreeHandler.ts changes:

Current flow (lines 90-98):
  addStatusMessage(ctx, `✓ Merged: ...`);
  ctx.cleanupCurrentSessionHandler();
  await destroySession(ctx.currentSessionId);
  ctx.onExit();

New flow:
  // Show rich file-by-file summary first
  addStatusMessage(ctx, buildMergeSummary(mergeResult));
  // Don't exit. Set action prompt instead.
  ctx.setActionPrompt({
    message: 'Merge complete — Press Enter to close session',
    onConfirm: async () => {
      ctx.cleanupCurrentSessionHandler();
      await destroySession(ctx.currentSessionId);
      ctx.onExit();
    }
  });

MergeWorktreeContext gains:
  setActionPrompt: (prompt: ActionPrompt | null) => void

AgentView passes its setActionPrompt state setter through the handler context.

Conflict path is unchanged — shows error message, no action prompt, session stays open.
  Rich merge summary format (added to conversation before action prompt):

✓ Merge successful

  Modified (3):
    src/auth/login.ts
    src/auth/register.ts
    src/utils/helpers.ts

  Added (1):
    src/auth/types.ts

  Deleted (0)

Conflict summary format:

⚠ Merge conflicts detected

  Conflicting files:
    src/auth/login.ts
    src/utils/helpers.ts

  These files were modified in both this session and the main worktree.
  Resolve the conflicts, then run /merge-worktree again.

Data sources: MergeResultJs.filesModified/Added/Deleted for success.

Conflict file parsing: The Rust error format uses Debug trait on Vec<String>: 'Conflict detected: ["file1.ts", "file2.ts"] have been modified in both session and main worktree'. Parse with best-effort regex to extract file paths. If parsing fails (e.g. Rust format changes), fall back to displaying the raw error.message — this is a known coupling with Rust's thiserror/Debug output and graceful degradation is acceptable.
  InputTransition.tsx changes:

New props:
  actionPrompt?: { message: string; onConfirm: () => void | Promise<void> } | null
  clearActionPrompt?: () => void

Rendering priority (early return order, BEFORE loading/animation phases):
  1. isPaused && pauseInfo → existing pause rendering (unchanged)
  2. actionPrompt → NEW action prompt rendering (green message + dimColor hint)
  3. animationPhase === 'loading' → existing loading/compaction (unchanged)
  4. ... rest of existing animation phases (unchanged)

Keyboard handler: Register via useInputCompat when actionPrompt is active, priority MEDIUM.
  Enter or Escape → await actionPrompt.onConfirm(), then clearActionPrompt()
  All other keys → consumed (blocked), return true

The handler must guard against double-invocation (user presses Enter twice fast). Use a local ref isClosing to prevent re-entry:
  const isClosingRef = useRef(false);
  if (isClosingRef.current) return true;
  isClosingRef.current = true;
  await actionPrompt.onConfirm();
  clearActionPrompt?.();

Note: The await in the keyboard handler requires the handler function to be async. useInputCompat's handler type supports this — same pattern as other async handlers in the codebase.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On successful merge, show the merge summary in the conversation AND put the input area into an 'action prompt' mode showing '✓ Merge complete — Press Enter to close session'. Only when Enter is pressed do cleanup/destroy/exit run.
  #   2. The merge summary in the conversation must list individual file paths grouped by category: Modified (yellow), Added (green), Deleted (red) — not just counts.
  #   3. While in action prompt mode, character input and Backspace are blocked (like compaction mode). Only Enter and Escape trigger the deferred close action.
  #   4. On conflict, show a detailed summary listing conflicting file paths in red plus guidance: 'Resolve these conflicts in your session, then run /merge-worktree again'. Input returns to normal mode (no action prompt).
  #   5. The 'action prompt' is a generic mechanism on InputTransition/MultiLineInput — not merge-specific. It takes a message string and an onConfirm callback. Other features can reuse it for deferred-action scenarios.
  #   6. Escape while in action prompt mode triggers the same close action as Enter — the user must not get stuck.
  #   7. 'Nothing to merge' path is unchanged — shows status message, session stays open, no action prompt.
  #   8. Non-conflict merge errors (e.g. WorktreeNotFound) show a generic error message, keep the session open, and do NOT trigger the action prompt.
  #   9. After calling onConfirm, the InputTransition keyboard handler automatically clears the action prompt (sets it to null). Callers do NOT need to clear it themselves — whether onConfirm unmounts the component or not, the prompt is always cleaned up.
  #
  # EXAMPLES:
  #   1. User runs /merge-worktree in isolated session with 3 modified, 1 added, 0 deleted files → sees file-by-file summary in conversation → input shows '✓ Merge complete — Press Enter to close session' → presses Enter → session closes, returns to board
  #   2. User runs /merge-worktree → merge succeeds → sees summary and 'Press Enter to close session' → presses Escape instead of Enter → session still closes and returns to board (Escape is equivalent)
  #   3. User runs /merge-worktree → merge succeeds → sees action prompt → types characters → nothing happens (input is blocked) → presses Enter → session closes
  #   4. User runs /merge-worktree → merge has conflicts on src/auth/login.ts and src/utils/helpers.ts → sees 'Merge conflicts detected' with file list in red and guidance text → input returns to normal mode → user can continue working and retry
  #   5. User runs /merge-worktree in session with no changes → sees 'Nothing to merge' → session stays open, input stays normal — no action prompt shown
  #   6. User runs /merge-worktree → worktree was already cleaned up by another process → sees 'Merge failed: Worktree not found' → session stays open, input stays normal — no action prompt shown
  #
  # ========================================

  Background: User Story
    As a developer using an isolated session
    I want to see a clear summary of what was merged and confirm before the session closes
    So that I'm not disoriented by the view suddenly switching to the board without seeing results

  @success-path
  Scenario: Successful merge shows file-by-file summary and action prompt
    Given I am in an isolated session with 3 modified, 1 added, and 0 deleted files
    When I run "/merge-worktree"
    Then I should see a merge summary in the conversation listing file paths grouped by category
    And the Modified files should be listed with their paths
    And the Added files should be listed with their paths
    And the Deleted count should show 0
    And the input area should show "✓ Merge complete — Press Enter to close session"
    When I press Enter
    Then the session should be cleaned up and destroyed
    And I should return to the board view

  @success-path
  Scenario: Escape in action prompt closes session same as Enter
    Given I am in an isolated session with changes
    And I have run "/merge-worktree" successfully
    And the input area shows the action prompt
    When I press Escape
    Then the session should be cleaned up and destroyed
    And I should return to the board view

  @input-blocking
  Scenario: Character input is blocked during action prompt
    Given I am in an isolated session with changes
    And I have run "/merge-worktree" successfully
    And the input area shows the action prompt
    When I type characters on the keyboard
    Then nothing should happen in the input area
    And the action prompt should remain visible
    When I press Enter
    Then the session should be cleaned up and destroyed

  @conflict-path
  Scenario: Merge conflicts show detailed file list and guidance
    Given I am in an isolated session
    And the session has conflicting changes with the main worktree on "src/auth/login.ts" and "src/utils/helpers.ts"
    When I run "/merge-worktree"
    Then I should see "Merge conflicts detected" in the conversation
    And the conflicting file paths should be listed
    And I should see guidance text "Resolve the conflicts, then run /merge-worktree again"
    And the input should return to normal mode
    And no action prompt should be shown

  @no-changes
  Scenario: Nothing to merge keeps session open without action prompt
    Given I am in an isolated session with no changes
    When I run "/merge-worktree"
    Then I should see "Nothing to merge" in the conversation
    And the session should stay open
    And the input should remain in normal mode
    And no action prompt should be shown

  @error-path
  Scenario: Non-conflict error keeps session open without action prompt
    Given I am in an isolated session
    And the worktree has been cleaned up by another process
    When I run "/merge-worktree"
    Then I should see "Merge failed: Worktree not found" in the conversation
    And the session should stay open
    And the input should remain in normal mode
    And no action prompt should be shown

  @generic-mechanism
  Scenario: Action prompt is a generic reusable mechanism
    Given I have an InputTransition component
    When I set an action prompt with message "Custom action" and an onConfirm callback
    Then the input area should display the message with a close hint
    And character input should be blocked
    And pressing Enter should invoke the onConfirm callback
    And the action prompt should be automatically cleared after onConfirm

  @double-invoke-guard
  Scenario: Action prompt guards against double invocation
    Given I am in an isolated session with changes
    And I have run "/merge-worktree" successfully
    And the input area shows the action prompt
    When I press Enter twice rapidly
    Then the onConfirm callback should only execute once
