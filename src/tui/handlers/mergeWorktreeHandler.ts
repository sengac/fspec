/**
 * /merge-worktree slash command handler
 *
 * Extracted from AgentView.tsx for separation of concerns and testability.
 * Merges worktree changes back to main project and closes the session.
 *
 * GIT-036, GIT-037
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

// Re-export ActionPrompt for consumers that import from the handler
export type { ActionPrompt } from '../types/actionPrompt';

/**
 * Dependencies injected from AgentView to enable testing without React.
 */
export interface MergeWorktreeContext {
  isIsolated: boolean;
  currentSessionId: string | null;
  repoPath: string;
  setConversation: (
    updater: (
      prev: Array<{ type: string; content: string }>
    ) => Array<{ type: string; content: string }>
  ) => void;
  setInputValue: (value: string) => void;
  cleanupCurrentSessionHandler: () => void;
  onExit: () => void;
  setActionPrompt: (prompt: ActionPrompt | null) => void;
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
      // GIT-037: Show rich conflict summary with file paths and guidance
      addStatusMessage(ctx, buildConflictSummary(errorMessage));
    } else {
      addStatusMessage(ctx, `Merge failed: ${errorMessage}`);
    }
  }
}
