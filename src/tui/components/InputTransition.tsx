/**
 * InputTransition - Animated transition between loading and input states
 *
 * Handles smooth character-by-character animation when transitioning from
 * "Thinking..." indicator to the input placeholder text.
 *
 * Animation sequence:
 * 1. When isLoading becomes false, animate out the thinking text (right to left)
 * 2. Then animate in the placeholder text (left to right reveal)
 * 3. User can interrupt animation by typing - immediately shows input
 *
 * Architecture:
 * - Animation constants centralized in animationConstants.ts
 * - Animation timing synchronized with Ink's render throttle (60fps)
 * - Multiple characters per frame for faster animation while staying smooth
 * - INPUT-001: Uses centralized input handling with MEDIUM priority
 */

import React, { useState, useEffect, useRef } from 'react';
import { Text } from 'ink';
import { useThinkingText } from './ThinkingIndicator';
import { MultiLineInput, type MultiLineInputProps } from './MultiLineInput';
import { CHAR_ANIMATION_INTERVAL_MS, ANIMATION_PHASE_DELAY_MS, CHARS_PER_FRAME } from '../utils/animationConstants';
import type { PauseInfo } from '../types/pause';
import { useInputCompat, InputPriority } from '../input/index';
import { logger } from '../../utils/logger';

// Re-export PauseInfo for backwards compatibility with existing imports
export type { PauseInfo } from '../types/pause';

type AnimationPhase = 'loading' | 'paused' | 'hiding' | 'showing' | 'complete';

export interface InputTransitionProps extends MultiLineInputProps {
  /**
   * Whether the AI is currently processing
   */
  isLoading: boolean;

  /**
   * The thinking message to display during loading
   * @default "Thinking"
   */
  thinkingMessage?: string;

  /**
   * The thinking hint text
   * @default "(Esc to stop)"
   */
  thinkingHint?: string;

  /**
   * Skip the transition animation and immediately show the input
   * Useful when switching sessions where animation would be jarring
   */
  skipAnimation?: boolean;

  /**
   * Whether the session is currently paused (tool pause feature)
   * When true, shows pause indicator instead of thinking spinner
   */
  isPaused?: boolean;

  /**
   * Information about the current pause state
   * Only used when isPaused is true
   */
  pauseInfo?: PauseInfo;
}

/**
 * Animated input transition component
 *
 * Wraps ThinkingIndicator and MultiLineInput with smooth character
 * animation when transitioning between loading and input states.
 * 
 * User can interrupt the animation by pressing any key - the animation
 * will immediately complete and the input will be focused with the
 * typed character captured.
 *
 * Animation is synchronized with Ink's 60fps render throttle.
 * Uses CHARS_PER_FRAME to control animation speed (default: 3 chars/frame).
 */
export const InputTransition: React.FC<InputTransitionProps> = ({
  isLoading,
  thinkingMessage = 'Thinking',
  thinkingHint = "(Esc to stop | 'Space+Esc' detach)",
  placeholder = "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn | 'Space+Esc' detach)",
  value,
  onChange,
  onSubmit,
  onHistoryPrev,
  onHistoryNext,
  onSessionPrev,
  onSessionNext,
  maxVisibleLines = 5,
  isActive = true,
  skipAnimation = false,
  isPaused = false,
  pauseInfo,
  suppressEnter = false,
}) => {
  // All useState hooks grouped together
  const [animationPhase, setAnimationPhase] = useState<AnimationPhase>(
    isLoading ? 'loading' : 'complete'
  );
  const [visibleChars, setVisibleChars] = useState(0);
  const [capturedText, setCapturedText] = useState('');
  const [pendingInput, setPendingInput] = useState('');
  const wasLoadingRef = useRef(isLoading);

  // Get the current thinking text (stays in sync with ThinkingIndicator)
  const currentThinkingText = useThinkingText(
    thinkingMessage,
    thinkingHint,
    'dots',
    isLoading
  );

  // TUI-049: Skip animation when requested (e.g., session switching)
  // Handles two cases:
  // 1. Animation already in progress (hiding/showing) - skip to complete
  // 2. isLoading changes while skipAnimation is true - go straight to complete
  useEffect(() => {
    if (!skipAnimation) return;

    // If mid-animation, skip to complete immediately
    if (animationPhase === 'hiding' || animationPhase === 'showing') {
      setAnimationPhase('complete');
    }
  }, [skipAnimation, animationPhase]);

  // Track loading state changes to trigger animation
  // PAUSE-001: Don't trigger hiding animation when entering paused state
  useEffect(() => {
    if (wasLoadingRef.current && !isLoading && !isPaused) {
      // Loading just finished (and NOT entering pause state)
      if (skipAnimation) {
        // TUI-049: Skip animation when switching sessions - go straight to complete
        setAnimationPhase('complete');
        setPendingInput('');
      } else {
        // Normal case: capture current text and start hide animation
        setCapturedText(currentThinkingText);
        setAnimationPhase('hiding');
        setVisibleChars(currentThinkingText.length);
        setPendingInput('');
      }
    } else if (!wasLoadingRef.current && isLoading) {
      // Loading just started (or resuming from pause)
      setAnimationPhase('loading');
      setVisibleChars(0);
      setCapturedText('');
      setPendingInput('');
    }
    wasLoadingRef.current = isLoading;
  }, [isLoading, isPaused, currentThinkingText, skipAnimation]);

  // Handle keyboard input during animation to interrupt it
  const isAnimating = animationPhase === 'hiding' || animationPhase === 'showing';
  
  useInputCompat({
    id: 'input-transition-animation',
    priority: InputPriority.MEDIUM,
    description: 'Input transition animation interrupt handler',
    isActive: isActive && isAnimating,
    handler: (input, key) => {
      // Ignore control keys that shouldn't trigger input capture
      if (key.escape || key.ctrl || key.meta) {
        return false;
      }
      
      // For Enter/Return: complete animation but let it propagate to input for submit
      // This ensures the user's submit action isn't lost during animation
      if (key.return) {
        setAnimationPhase('complete');
        return false; // Let Enter propagate to MultiLineInput for submit
      }
      
      // For backspace/delete: complete animation but let it propagate
      if (key.backspace || key.delete) {
        setAnimationPhase('complete');
        return false; // Let it propagate to handle the delete action
      }
      
      // For printable characters, capture them and skip animation
      // These characters will be appended to the input value
      if (input && input.length > 0) {
        // Capture the typed character to pass to input
        setPendingInput(input);
        // Immediately complete the animation
        setAnimationPhase('complete');
        return true;
      }

      return false;
    },
  });

  // When animation completes with pending input, update the value
  useEffect(() => {
    if (animationPhase === 'complete' && pendingInput && onChange) {
      // Append the pending input to the current value
      onChange(value + pendingInput);
      setPendingInput('');
    }
  }, [animationPhase, pendingInput, onChange, value]);

  // Handle hiding animation (right to left) - multiple chars per frame
  useEffect(() => {
    if (animationPhase !== 'hiding') {
      return;
    }

    if (visibleChars <= 0) {
      // Hiding complete, start showing phase after delay
      const delayTimer = setTimeout(() => {
        setAnimationPhase('showing');
        setVisibleChars(0);
      }, ANIMATION_PHASE_DELAY_MS);
      return () => clearTimeout(delayTimer);
    }

    const timer = setTimeout(() => {
      // Decrement by CHARS_PER_FRAME, but don't go below 0
      setVisibleChars((prev) => Math.max(0, prev - CHARS_PER_FRAME));
    }, CHAR_ANIMATION_INTERVAL_MS);

    return () => clearTimeout(timer);
  }, [animationPhase, visibleChars]);

  // Handle showing animation (left to right reveal of placeholder) - multiple chars per frame
  useEffect(() => {
    if (animationPhase !== 'showing') {
      return;
    }

    if (visibleChars >= placeholder.length) {
      // Animation complete
      setAnimationPhase('complete');
      return;
    }

    const timer = setTimeout(() => {
      // Increment by CHARS_PER_FRAME, but don't exceed length
      setVisibleChars((prev) => Math.min(placeholder.length, prev + CHARS_PER_FRAME));
    }, CHAR_ANIMATION_INTERVAL_MS);

    return () => clearTimeout(timer);
  }, [animationPhase, visibleChars, placeholder.length]);

  // Render based on current state
  
  // Show pause indicator when paused
  if (isPaused && pauseInfo) {
    if (pauseInfo.kind === 'confirm') {
      // Confirm pause: show warning with Y/N options
      return (
        <Text>
          <Text color="yellow">⏸ {pauseInfo.toolName}</Text>
          <Text>: </Text>
          <Text color="yellow">{pauseInfo.message}</Text>
          {pauseInfo.details && (
            <Text>
              {'\n'}
              <Text dimColor>  {pauseInfo.details}</Text>
            </Text>
          )}
          <Text>
            {'\n'}
            <Text color="green">[Y] Approve</Text>
            <Text> </Text>
            <Text color="red">[N] Deny</Text>
            <Text dimColor> (Esc to cancel)</Text>
          </Text>
        </Text>
      );
    } else {
      // Continue pause: show pause indicator with Enter hint
      return (
        <Text>
          <Text color="cyan">⏸ {pauseInfo.toolName}</Text>
          <Text>: </Text>
          <Text>{pauseInfo.message}</Text>
          <Text dimColor> (Press Enter to continue)</Text>
        </Text>
      );
    }
  }
  
  if (animationPhase === 'loading') {
    // Show the live thinking text (stays in sync with the hook)
    return <Text dimColor>{currentThinkingText}</Text>;
  }

  if (animationPhase === 'hiding') {
    // Animate hiding from right to left using captured text
    const visibleText = capturedText.slice(0, visibleChars);
    return <Text dimColor>{visibleText}</Text>;
  }

  if (animationPhase === 'showing') {
    // Animate showing from left to right (reveal from start)
    const visiblePart = placeholder.slice(0, visibleChars);
    return <Text dimColor>{visiblePart}</Text>;
  }

  // Animation complete - show full input
  return (
    <MultiLineInput
      value={value}
      onChange={onChange}
      onSubmit={onSubmit}
      placeholder={placeholder}
      onHistoryPrev={onHistoryPrev}
      onHistoryNext={onHistoryNext}
      maxVisibleLines={maxVisibleLines}
      isActive={isActive}
      suppressEnter={suppressEnter}
    />
  );
};

export default InputTransition;
