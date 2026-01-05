/**
 * ThinkingIndicator - Animated thinking indicator for AI agent loading states
 *
 * Implements animated "Thinking..." text with spinner for the AgentView input area.
 * Based on patterns from opencode, vtcode, and ink-spinner.
 *
 * Animation styles:
 * - 'dots': Cycles dots: "Thinking.", "Thinking..", "Thinking..."
 * - 'spinner': Uses braille spinner characters
 * - 'bounce': Bouncing dots animation
 */

import React, { useState, useEffect } from 'react';
import { Text } from 'ink';

// Spinner frame definitions (from cli-spinners patterns)
export const SPINNERS = {
  // Braille dots spinner (same as opencode's Pulse)
  dots: {
    frames: ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
    interval: 80,
  },
  // Classic dots animation
  ellipsis: {
    frames: ['.  ', '.. ', '...', '   '],
    interval: 300,
  },
  // Bouncing ball
  bounce: {
    frames: ['⠁', '⠂', '⠄', '⠂'],
    interval: 120,
  },
  // Simple line spinner
  line: {
    frames: ['-', '\\', '|', '/'],
    interval: 100,
  },
  // Braille arc
  arc: {
    frames: ['◜', '◠', '◝', '◞', '◡', '◟'],
    interval: 100,
  },
  // Growing bars
  growVertical: {
    frames: ['▁', '▃', '▄', '▅', '▆', '▇', '▆', '▅', '▄', '▃'],
    interval: 80,
  },
} as const;

type SpinnerType = keyof typeof SPINNERS;

export interface ThinkingIndicatorProps {
  /**
   * The message to display alongside the spinner
   * @default "Thinking"
   */
  message?: string;

  /**
   * Additional hint text shown after the message
   * @default "(Esc to stop)"
   */
  hint?: string;

  /**
   * Spinner animation type
   * @default "dots"
   */
  type?: SpinnerType;

  /**
   * Whether the indicator is active (animating)
   * @default true
   */
  isActive?: boolean;

  /**
   * Text color for the message
   * @default undefined (dimColor applied)
   */
  color?: string;
}

/**
 * Animated thinking indicator component
 *
 * Usage:
 * ```tsx
 * <ThinkingIndicator />
 * <ThinkingIndicator message="Processing" type="spinner" />
 * <ThinkingIndicator message="Loading" hint="(press any key)" />
 * ```
 */
export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({
  message = 'Thinking',
  hint = '(Esc to stop)',
  type = 'dots',
  isActive = true,
  color,
}) => {
  const [frame, setFrame] = useState(0);
  const spinner = SPINNERS[type];

  useEffect(() => {
    if (!isActive) {
      return;
    }

    const timer = setInterval(() => {
      setFrame((prevFrame) => {
        const isLastFrame = prevFrame === spinner.frames.length - 1;
        return isLastFrame ? 0 : prevFrame + 1;
      });
    }, spinner.interval);

    return () => {
      clearInterval(timer);
    };
  }, [spinner, isActive]);

  // Reset frame when spinner type changes
  useEffect(() => {
    setFrame(0);
  }, [type]);

  if (!isActive) {
    return null;
  }

  const spinnerChar = spinner.frames[frame];

  return (
    <Text dimColor={!color} color={color}>
      {spinnerChar} {message}... {hint}
    </Text>
  );
};

/**
 * Hook to get the current thinking indicator text
 * Used by InputTransition to capture the exact displayed text
 */
export const useThinkingText = (
  message: string = 'Thinking',
  hint: string = '(Esc to stop)',
  type: SpinnerType = 'dots',
  isActive: boolean = true
): string => {
  const [frame, setFrame] = useState(0);
  const spinner = SPINNERS[type];

  useEffect(() => {
    if (!isActive) {
      return;
    }

    const timer = setInterval(() => {
      setFrame((prevFrame) => {
        const isLastFrame = prevFrame === spinner.frames.length - 1;
        return isLastFrame ? 0 : prevFrame + 1;
      });
    }, spinner.interval);

    return () => {
      clearInterval(timer);
    };
  }, [spinner, isActive]);

  useEffect(() => {
    setFrame(0);
  }, [type]);

  const spinnerChar = spinner.frames[frame];
  return `${spinnerChar} ${message}... ${hint}`;
};

export default ThinkingIndicator;
