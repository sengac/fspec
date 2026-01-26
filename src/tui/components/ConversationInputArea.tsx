/**
 * ConversationInputArea - Reusable input area for conversation views
 *
 * Encapsulates the input area with border, prompt character, and InputTransition.
 * Used by both AgentView (main conversation) and SplitSessionView (watcher).
 *
 * SOLID: Single responsibility - handles input display and submission
 * DRY: Reused across different conversation views
 */

import React from 'react';
import { Box, Text } from 'ink';
import { InputTransition } from './InputTransition';
import type { PauseInfo } from '../types/pause';

// Re-export for convenience
export type { PauseInfo } from '../types/pause';

export interface ConversationInputAreaProps {
  /** Current input value */
  value: string;
  /** Callback when input value changes */
  onChange: (value: string) => void;
  /** Callback when user submits (Enter) */
  onSubmit: (value: string) => void;
  /** Whether the AI is currently processing */
  isLoading: boolean;
  /** Placeholder text when input is empty */
  placeholder?: string;
  /** Whether input is active (focused) */
  isActive?: boolean;
  /** Skip the loading->input animation */
  skipAnimation?: boolean;
  /** Callback for Shift+Up (history prev) - optional */
  onHistoryPrev?: () => void;
  /** Callback for Shift+Down (history next) - optional */
  onHistoryNext?: () => void;
  /** Callback for Shift+Left (session prev) - optional */
  onSessionPrev?: () => void;
  /** Callback for Shift+Right (session next) - optional */
  onSessionNext?: () => void;
  /** Maximum visible lines for multi-line input */
  maxVisibleLines?: number;
  /** Prompt character/string (default: "> ") */
  promptChar?: string;
  /** Prompt color (default: "green") */
  promptColor?: string;
  /** PAUSE-001: Whether the session is currently paused */
  isPaused?: boolean;
  /** PAUSE-001: Information about the current pause state */
  pauseInfo?: PauseInfo;
}

/**
 * ConversationInputArea component
 *
 * Renders a bordered input area with:
 * - Colored prompt character
 * - InputTransition (handles loading animation and MultiLineInput)
 */
export const ConversationInputArea: React.FC<ConversationInputAreaProps> = ({
  value,
  onChange,
  onSubmit,
  isLoading,
  placeholder = "Type a message...",
  isActive = true,
  skipAnimation = false,
  onHistoryPrev,
  onHistoryNext,
  onSessionPrev,
  onSessionNext,
  maxVisibleLines = 5,
  promptChar = '> ',
  promptColor = 'green',
  isPaused = false,
  pauseInfo,
}) => {
  return (
    <Box
      borderStyle="single"
      borderTop={true}
      borderBottom={false}
      borderLeft={false}
      borderRight={false}
      paddingX={1}
    >
      <Text color={promptColor}>{promptChar}</Text>
      <Box flexGrow={1}>
        <InputTransition
          isLoading={isLoading}
          value={value}
          onChange={onChange}
          onSubmit={onSubmit}
          placeholder={placeholder}
          onHistoryPrev={onHistoryPrev}
          onHistoryNext={onHistoryNext}
          onSessionPrev={onSessionPrev}
          onSessionNext={onSessionNext}
          maxVisibleLines={maxVisibleLines}
          isActive={isActive}
          skipAnimation={skipAnimation}
          isPaused={isPaused}
          pauseInfo={pauseInfo}
        />
      </Box>
    </Box>
  );
};
