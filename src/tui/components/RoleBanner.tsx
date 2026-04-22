/**
 * RoleBanner - Displays the active session role below SessionHeader
 *
 * TUI-081: When a session has an active role (set via /role or set_role),
 * this component renders a single-line banner showing the role text.
 * When no role is set, nothing is rendered (zero height).
 *
 * Layout when active:
 *   #1 (AUTH-001: implementing): claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
 *   Role: You are a security reviewer. Analyze code for vulnerabilities...
 *   ───────────────────────────────────────────────────────────────────────────────
 *   [conversation area]
 *
 * Layout when inactive:
 *   #1 (AUTH-001: implementing): claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
 *   ───────────────────────────────────────────────────────────────────────────────
 *   [conversation area]
 */

import React from 'react';
import { Box, Text } from 'ink';
import chalk from 'chalk';

export interface RoleBannerProps {
  /** The role text to display, or null/empty if no role is set */
  roleText: string | null;
}

export const RoleBanner: React.FC<RoleBannerProps> = ({ roleText }) => {
  if (!roleText) {
    return null;
  }

  // Collapse all whitespace (including newlines) into single spaces so the
  // banner always renders as a single line. Without this, multi-line role
  // prompts cause Ink to render multiple lines even with `wrap="truncate-end"`,
  // since truncation applies per-line rather than to the entire block.
  const singleLineRole = roleText.replace(/\s+/g, ' ').trim();

  return (
    <Box height={1} width="100%" flexShrink={0} overflow="hidden">
      <Text wrap="truncate-end">
        {chalk.cyan('Role:')} {chalk.dim(singleLineRole)}
      </Text>
    </Box>
  );
};
