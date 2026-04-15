/**
 * SessionFooter - Status bar showing CWD and git branch name
 *
 * Displays the current working directory and git branch name only.
 * No dirty/untracked status indicators — just the branch name.
 *
 * Layout (right-aligned):
 *   ~/projects/fspec [⎇ main]
 *
 * Uses a dark grey background (#333333) matching SessionHeader to visually
 * bookend the conversation area.
 *
 * STATE FROM RUST VIA ZUSTAND: This component reads from the footerStore,
 * which is populated by FooterStateUpdate events emitted by a Rust background
 * poller (every 5 seconds). The data flow is:
 *
 *   Rust (tokio task, 5s) → reads .git/HEAD for branch name → FooterStateUpdate chunk
 *   → GlobalSessionStreamManager → footerStore.updateFooterState()
 *   → SessionFooter re-renders via Zustand selector
 *
 * The Rust poller ONLY reads the branch name (near-zero CPU cost).
 * It does NOT call get_staged_files, get_unstaged_files, or get_untracked_files.
 *
 * ZERO TypeScript-side polling. ZERO NAPI calls from the render path.
 */

import React from 'react';
import { Box, Text } from 'ink';
import chalk from 'chalk';
import { useSessionFooterState } from '../store/footerStore';

export interface SessionFooterProps {
  /** Session ID to show CWD and git info for */
  sessionId: string | null;
}

/**
 * Format git branch display — branch name only, no status indicators
 * @returns e.g. "[⎇ main]", "[⎇ (detached)]"
 */
const formatBranchDisplay = (branch: string | null): string => {
  const branchName = branch ?? '(detached)';
  return `[⎇ ${branchName}]`;
};

export const SessionFooter: React.FC<SessionFooterProps> = ({ sessionId }) => {
  const footerState = useSessionFooterState(sessionId);

  // Don't render content if no session or no CWD yet (Rust poller hasn't emitted)
  if (!footerState.displayPath) {
    return (
      <Box height={1} width="100%" flexDirection="row" backgroundColor="#333333" paddingLeft={1} paddingRight={1}>
        <Box flexGrow={1} flexShrink={1} minWidth={0} />
      </Box>
    );
  }

  // Build right-side content as a single chalk-styled string
  let rightContent = chalk.dim(footerState.displayPath);

  if (footerState.git.isGitRepo) {
    rightContent +=
      ' ' +
      chalk.cyan(formatBranchDisplay(footerState.git.branch));
  }

  return (
    <Box height={1} width="100%" flexDirection="row" backgroundColor="#333333" paddingLeft={1} paddingRight={1}>
      {/* Left side spacer — pushes content to the right */}
      <Box flexGrow={1} flexShrink={1} minWidth={0} />

      {/* Right side: CWD and branch info, never shrink */}
      <Box flexShrink={0}>
        <Text wrap="truncate-end">{rightContent}</Text>
      </Box>
    </Box>
  );
};
