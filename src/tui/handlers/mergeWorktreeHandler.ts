/**
 * /merge-worktree slash command handler
 *
 * Extracted from AgentView.tsx for separation of concerns and testability.
 * Merges worktree changes back to main project and closes the session.
 *
 * GIT-036, GIT-037, GIT-038
 */

import {
  inspectSessionChanges,
  mergeSessionChanges,
  destroySession,
} from '../services/sessionService';
import type { ActionPrompt } from '../types/actionPrompt';
import {
  buildMergeSummary,
  buildConflictSummary,
} from './mergeSummaryFormatting';
import { buildConflictLlmContext } from './conflictLlmContext';

// Re-export ActionPrompt for consumers that import from the handler
export type { ActionPrompt } from '../types/actionPrompt';

/**
 * Dependencies injected from AgentView to enable testing without React.
 */
export interface MergeWorktreeContext {
  isIsolated: boolean;
  currentSessionId: string | null;
  repoPath: string;
  /**
   * GIT-038: The worktree path for isolated sessions.
   * Used to tell the LLM where conflicting files are located.
   */
  worktreePath: string | null;
  setConversation: (
    updater: (
      prev: Array<{ type: string; content: string }>
    ) => Array<{ type: string; content: string }>
  ) => void;
  setInputValue: (value: string) => void;
  cleanupCurrentSessionHandler: () => void;
  onExit: () => void;
  setActionPrompt: (prompt: ActionPrompt | null) => void;
  /**
   * GIT-038: Inject a context message into the LLM's session history.
   * This is separate from setConversation (which is UI-only).
   * The AgentView wires this to persist the message as assistant-role
   * in both the Rust session and the React conversation state.
   */
  injectLlmContext: (content: string) => void;
}

/**
 * Append a status message to conversation.
 */
function addStatusMessage(ctx: MergeWorktreeContext, content: string): void {
  ctx.setConversation(prev => [...prev, { type: 'status', content }]);
}

/**
 * Handle the /merge-worktree command.
 *
 * Flow:
 * 1. Check isIsolated - show error if not isolated
 * 2. Check session exists
 * 3. Inspect session for changes - show "Nothing to merge" if empty
 * 4. Merge changes - on success: show rich summary, set action prompt
 * 5. On conflict: show conflict details with guidance, keep session open
 * 6. On other error: show generic error, keep session open
 */
export async function handleMergeWorktree(
  ctx: MergeWorktreeContext
): Promise<void> {
  ctx.setInputValue('');

  if (!ctx.isIsolated) {
    addStatusMessage(
      ctx,
      'This command is only available in isolated sessions'
    );
    return;
  }

  if (!ctx.currentSessionId) {
    addStatusMessage(ctx, 'No active session');
    return;
  }

  try {
    // Inspect session to check for changes before merging
    const inspectResult = inspectSessionChanges(
      ctx.repoPath,
      ctx.currentSessionId
    );
    const totalChanges =
      inspectResult.filesChanged.length +
      inspectResult.filesAdded.length +
      inspectResult.filesDeleted.length;

    if (totalChanges === 0) {
      addStatusMessage(ctx, 'Nothing to merge');
      return;
    }

    // Attempt merge
    const mergeResult = mergeSessionChanges(ctx.repoPath, ctx.currentSessionId);

    // GIT-037: Show rich file-by-file summary
    addStatusMessage(ctx, buildMergeSummary(mergeResult));

    // GIT-037: Set action prompt instead of immediately exiting
    const sessionId = ctx.currentSessionId;
    ctx.setActionPrompt({
      message: 'Merge complete — Press Enter to close session',
      onConfirm: async () => {
        ctx.cleanupCurrentSessionHandler();
        await destroySession(sessionId);
        ctx.onExit();
      },
    });
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (errorMessage.includes('Conflict')) {
      // GIT-037: Show rich conflict summary in TUI (visual, for user's eyes)
      addStatusMessage(ctx, buildConflictSummary(errorMessage));

      // GIT-038: Inject context into LLM conversation history so the AI
      // knows about the conflict and can help resolve it
      if (ctx.currentSessionId) {
        ctx.injectLlmContext(
          buildConflictLlmContext(errorMessage, ctx.worktreePath)
        );
      }
    } else {
      addStatusMessage(ctx, `Merge failed: ${errorMessage}`);
    }
  }
}
