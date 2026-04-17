/**
 * SessionHeader - Shared header component for session views
 *
 * Displays model info, capability indicators, and token usage.
 * Used by AgentView.
 *
 * Work unit ID and status are read from Zustand sessionStore (not props).
 *
 * Normal mode (with session number and work unit):
 *   #1 (AUTH-001: implementing): claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
 *
 * Uses a dark grey background (#333333) to visually separate from the conversation area below.
 *
 * Badge Colors:
 *   - [R] = magenta (reasoning)
 *   - [V] = blue (vision)
 *   - [DEBUG] = red bold
 *   - [SELECT] = cyan
 *   - [T:*] = yellow (thinking level)
 *
 * IMPORTANT: The left side uses a SINGLE Text element with textWrap="truncate-end"
 * to avoid Ink's flex layout issues with dynamic content. When multiple Text elements
 * are used as flex children with overflow="hidden", position calculation can fail
 * when content changes dynamically (e.g., from Zustand updates). Using a single
 * Text element with chalk-styled content ensures proper truncation.
 */

import React from 'react';
import { Box, Text } from 'ink';
import chalk from 'chalk';
import {
  formatContextWindow,
  getContextFillColor,
  getMaxTokens,
  TokenTracker,
} from '../utils/sessionHeaderUtils';
import { JsThinkingLevel } from '@sengac/codelet-napi';
import { useCurrentWorkUnitId, useCurrentWorkUnitStatus } from '../store/sessionStore';

export interface SessionHeaderProps {
  /** Model ID to display */
  modelId: string;
  /** Whether model supports reasoning/extended thinking */
  hasReasoning?: boolean;
  /** Whether model supports vision */
  hasVision?: boolean;
  /** Model's context window size in tokens */
  contextWindow?: number;
  /** CTX-009: Compaction threshold from Rust (badge shows this instead of contextWindow) */
  compactionThreshold?: number;
  /** Whether debug capture is enabled */
  isDebugEnabled?: boolean;
  /** Whether turn select mode is active */
  isSelectMode?: boolean;
  /** Current thinking level (shown while streaming) */
  thinkingLevel?: JsThinkingLevel | null;
  /** TUI-054: Base thinking level from /thinking dialog (shown when idle) */
  baseThinkingLevel?: JsThinkingLevel;
  /** Whether AI is currently processing */
  isLoading?: boolean;
  /** Tokens per second (shown while streaming) */
  tokensPerSecond?: number | null;
  /** Token usage from streaming updates */
  tokenUsage?: TokenTracker;
  /** Token usage from Rust state */
  rustTokens?: TokenTracker;
  /** Context fill percentage (0-100) */
  contextFillPercentage?: number;
  /** Compaction reduction percentage (shown after compaction) */
  compactionReduction?: number | null;
  /** Session number (1-based index in session list) - helps identify session when switching */
  sessionNumber?: number;
  /** GIT-029: Whether session is isolated (has a git worktree) */
  isIsolated?: boolean;
}

/**
 * Get thinking level display label
 * TUI-054: Returns null for Off (no badge shown), label for other levels
 */
const getThinkingLevelLabel = (level: JsThinkingLevel): string | null => {
  switch (level) {
    case JsThinkingLevel.Off:
      return null;
    case JsThinkingLevel.Low:
      return '[T:Low]';
    case JsThinkingLevel.Medium:
      return '[T:Med]';
    case JsThinkingLevel.High:
      return '[T:High]';
    default:
      return null;
  }
};

/**
 * Format percentage with up to 2 decimal places, removing trailing zeros
 * @example 45.678 → "45.68", 50.0 → "50", 22.10 → "22.1"
 */
const formatPercentage = (num: number): string => {
  return parseFloat(num.toFixed(2)).toString();
};

export const SessionHeader: React.FC<SessionHeaderProps> = ({
  modelId,
  hasReasoning = false,
  hasVision = false,
  contextWindow = 0,
  compactionThreshold,
  isDebugEnabled = false,
  isSelectMode = false,
  thinkingLevel = null,
  baseThinkingLevel = JsThinkingLevel.Off,
  isLoading = false,
  tokensPerSecond = null,
  tokenUsage = { inputTokens: 0, outputTokens: 0 },
  rustTokens = { inputTokens: 0, outputTokens: 0 },
  contextFillPercentage = 0,
  compactionReduction = null,
  sessionNumber,
  isIsolated = false,
}) => {
  // TUI-060: Simple Zustand selectors for work unit info
  const workUnitId = useCurrentWorkUnitId();
  const workUnitStatus = useCurrentWorkUnitStatus();

  const { inputTokens, outputTokens, reasoningTokens } = getMaxTokens(tokenUsage, rustTokens);

  const percentText =
    compactionReduction !== null
      ? `[${formatPercentage(contextFillPercentage)}%: COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%]`
      : `[${formatPercentage(contextFillPercentage)}%]`;

  // TUI-054: Show thinking level badge only when level > Off
  // During loading: show the effective level (already computed in AgentView)
  // When idle: show base level (if set)
  const displayLevel = isLoading && thinkingLevel !== null ? thinkingLevel : baseThinkingLevel;
  const thinkingLabel = getThinkingLevelLabel(displayLevel);

  // Build the session/work unit prefix text
  const sessionPrefix = sessionNumber !== undefined ? `#${sessionNumber}` : '';
  const workUnitText = workUnitId ? ` (${workUnitId}${workUnitStatus ? `: ${workUnitStatus}` : ''})` : '';
  const separator = (sessionPrefix || workUnitId) ? ': ' : '';

  // Build the left side as a single styled string to avoid Ink flex layout issues
  // with multiple Text elements and overflow="hidden". Using chalk ensures ANSI
  // codes are handled properly by cli-truncate when textWrap="truncate-end".
  let leftContent = '';

  // Session number, work unit, and model - cyan bold
  leftContent += chalk.cyan.bold(`${sessionPrefix}${workUnitText}${separator}${modelId || 'Loading...'}`);

  // Badges - each with their own color
  if (isIsolated) {
    leftContent += chalk.green(' [ISOLATED]');
  }
  if (hasReasoning) {
    leftContent += chalk.magenta(' [R]');
  }
  if (hasVision) {
    leftContent += chalk.blue(' [V]');
  }
  // CTX-009: Badge shows compaction threshold (what fill% is relative to), not raw context window.
  // Falls back to contextWindow when threshold is unavailable (pre-model-selection).
  const badgeValue = compactionThreshold ?? contextWindow;
  if (badgeValue > 0) {
    leftContent += chalk.dim(` [${formatContextWindow(badgeValue)}]`);
  }
  if (isDebugEnabled) {
    leftContent += chalk.red.bold(' [DEBUG]');
  }
  if (isSelectMode) {
    leftContent += chalk.cyan(' [SELECT]');
  }
  if (thinkingLabel) {
    leftContent += chalk.yellow(` ${thinkingLabel}`);
  }

  return (
    <Box flexDirection="column" width="100%">
      <Box height={1} width="100%" flexDirection="row" backgroundColor="#333333" paddingLeft={1} paddingRight={1}>
        {/* Left side: single Text element with truncation to avoid flex positioning issues */}
        <Box flexGrow={1} flexShrink={1} minWidth={0}>
          <Text wrap="truncate-end">{leftContent}</Text>
        </Box>

        {/* Spacer */}
        <Text> </Text>

        {/* Right side: never shrink, always visible */}
        <Box flexShrink={0} flexDirection="row">
          {/* Tokens per second - magenta, only shown when loading */}
          {isLoading && tokensPerSecond !== null && (
            <Text color="magenta">{tokensPerSecond.toFixed(1)} tok/s  </Text>
          )}
          {/* Token counts - dimmed */}
          <Text dimColor>tokens: {inputTokens}↓ {outputTokens}↑{reasoningTokens > 0 ? ` ${reasoningTokens}🧠` : ''}  </Text>
          {/* Context fill percentage - color varies by fill level */}
          <Text color={getContextFillColor(contextFillPercentage)}>
            {percentText}
          </Text>
        </Box>
      </Box>
    </Box>
  );
};
