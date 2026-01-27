/**
 * SessionHeader - Shared header component for session views
 *
 * Displays model info, capability indicators, and token usage.
 * Used by both AgentView (normal mode) and SplitSessionView (watcher mode).
 *
 * SOLID: Single Responsibility - only header rendering
 * DRY: Shared between AgentView and SplitSessionView
 *
 * Normal mode (with session number):
 *   Agent #1: claude-sonnet-4 [R] [V] [200k]           1234↓ 567↑ [45%]
 *   ─────────────────────────────────────────────────────────────────
 *
 * Watcher mode:
 *   Watcher: security-reviewer #1 | Agent #2: claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
 *   ──────────────────────────────────────────────────────────────────────────────────────────
 *
 * Badge Colors:
 *   - [R] = magenta (reasoning)
 *   - [V] = blue (vision)
 *   - [DEBUG] = red bold
 *   - [SELECT] = cyan
 *   - [T:*] = yellow (thinking level)
 */

import React from 'react';
import { Box, Text } from 'ink';
import {
  formatContextWindow,
  getContextFillColor,
  getMaxTokens,
  TokenTracker,
} from '../utils/sessionHeaderUtils';
import { JsThinkingLevel } from '@sengac/codelet-napi';

/**
 * Watcher info for header display
 */
export interface WatcherHeaderInfo {
  /** Template slug (e.g., "security-reviewer") */
  slug: string;
  /** Instance number (1-based) */
  instanceNumber: number;
}

export interface SessionHeaderProps {
  /** Model ID to display */
  modelId: string;
  /** Whether model supports reasoning/extended thinking */
  hasReasoning?: boolean;
  /** Whether model supports vision */
  hasVision?: boolean;
  /** Model's context window size in tokens */
  contextWindow?: number;
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
  /** Watcher info - if present, shows watcher prefix */
  watcherInfo?: WatcherHeaderInfo;
  /** Session number (1-based index in session list) - helps identify session when switching */
  sessionNumber?: number;
  /** Work unit ID attached to this session (e.g., "TEST-001") */
  workUnitId?: string;
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

export const SessionHeader: React.FC<SessionHeaderProps> = ({
  modelId,
  hasReasoning = false,
  hasVision = false,
  contextWindow = 0,
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
  watcherInfo,
  sessionNumber,
  workUnitId,
}) => {
  const { inputTokens, outputTokens } = getMaxTokens(tokenUsage, rustTokens);

  const percentText =
    compactionReduction !== null
      ? `[${contextFillPercentage}%: COMPACTED -${compactionReduction}%]`
      : `[${contextFillPercentage}%]`;

  // TUI-054: Show thinking level badge only when level > Off
  // During loading: show the effective level (already computed in AgentView)
  // When idle: show base level (if set)
  const displayLevel = isLoading && thinkingLevel !== null ? thinkingLevel : baseThinkingLevel;
  const thinkingLabel = getThinkingLevelLabel(displayLevel);

  return (
    <Box flexDirection="column" width="100%">
      <Box height={1} width="100%" flexDirection="row" overflow="hidden">
        {/* Left side: model info and badges with proper colors */}
        <Box flexGrow={1} flexShrink={1} overflow="hidden">
          {/* Watcher prefix (if applicable) - blue text */}
          {watcherInfo && (
            <>
              <Text color="blue" bold>
                Watcher: {watcherInfo.slug} #{watcherInfo.instanceNumber}
              </Text>
              <Text> | </Text>
            </>
          )}
          {/* Agent label and model - show session number if available */}
          <Text color="cyan" bold>
            Agent{sessionNumber !== undefined ? ` #${sessionNumber}` : ''}{workUnitId ? ` (${workUnitId})` : ''}: {modelId}
          </Text>
          {/* Reasoning badge - magenta */}
          {hasReasoning && <Text color="magenta"> [R]</Text>}
          {/* Vision badge - blue */}
          {hasVision && <Text color="blue"> [V]</Text>}
          {/* Context window - dimColor */}
          {contextWindow > 0 && (
            <Text dimColor> [{formatContextWindow(contextWindow)}]</Text>
          )}
          {/* Debug badge - red bold */}
          {isDebugEnabled && (
            <Text color="red" bold>
              {' '}
              [DEBUG]
            </Text>
          )}
          {/* Select mode badge - cyan */}
          {isSelectMode && <Text color="cyan"> [SELECT]</Text>}
          {/* TUI-054: Thinking level badge - yellow, only shown when level > Off */}
          {thinkingLabel && <Text color="yellow"> {thinkingLabel}</Text>}
        </Box>

        {/* Spacer */}
        <Text> </Text>

        {/* Right side: never shrink, always visible */}
        <Box flexShrink={0}>
          {/* Tokens per second - magenta, only shown when loading */}
          {isLoading && tokensPerSecond !== null && (
            <Text color="magenta">{tokensPerSecond.toFixed(1)} tok/s  </Text>
          )}
          {/* Token counts - dimmed */}
          <Text dimColor>tokens: {inputTokens}↓ {outputTokens}↑  </Text>
          {/* Context fill percentage - color varies by fill level */}
          <Text color={getContextFillColor(contextFillPercentage)}>
            {percentText}
          </Text>
        </Box>
      </Box>
      {/* Bottom border separator */}
      <Box
        width="100%"
        borderStyle="single"
        borderBottom
        borderTop={false}
        borderLeft={false}
        borderRight={false}
      />
    </Box>
  );
};
