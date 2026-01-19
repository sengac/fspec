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
 */

import React, { useState, useEffect, useRef } from 'react';
import { Text, useInput } from 'ink';
import { useThinkingText } from './ThinkingIndicator';
import { MultiLineInput, type MultiLineInputProps } from './MultiLineInput';

// Animation timing constants
const CHAR_HIDE_INTERVAL = 3; // ms per character when hiding
const CHAR_SHOW_INTERVAL = 2; // ms per character when revealing
const TRANSITION_DELAY = 12; // ms delay between hide and show phases

type AnimationPhase = 'loading' | 'hiding' | 'showing' | 'complete';

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
  useEffect(() => {
    if (wasLoadingRef.current && !isLoading) {
      // Loading just finished
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
      // Loading just started
      setAnimationPhase('loading');
      setVisibleChars(0);
      setCapturedText('');
      setPendingInput('');
    }
    wasLoadingRef.current = isLoading;
  }, [isLoading, currentThinkingText, skipAnimation]);

  // Handle keyboard input during animation to interrupt it
  const isAnimating = animationPhase === 'hiding' || animationPhase === 'showing';
  
  useInput(
    (input, key) => {
      // Ignore control keys that shouldn't trigger input
      if (key.escape || key.ctrl || key.meta) {
        return;
      }
      
      // For printable characters, capture them and skip animation
      if (input && input.length > 0 && !key.return) {
        // Capture the typed character to pass to input
        setPendingInput(input);
        // Immediately complete the animation
        setAnimationPhase('complete');
      } else if (key.return || key.backspace || key.delete) {
        // For return/backspace/delete, just skip animation without capturing
        setAnimationPhase('complete');
      }
    },
    { isActive: isActive && isAnimating }
  );

  // When animation completes with pending input, update the value
  useEffect(() => {
    if (animationPhase === 'complete' && pendingInput && onChange) {
      // Append the pending input to the current value
      onChange(value + pendingInput);
      setPendingInput('');
    }
  }, [animationPhase, pendingInput, onChange, value]);

  // Handle hiding animation (right to left)
  useEffect(() => {
    if (animationPhase !== 'hiding') {
      return;
    }

    if (visibleChars <= 0) {
      // Hiding complete, start showing phase after delay
      const delayTimer = setTimeout(() => {
        setAnimationPhase('showing');
        setVisibleChars(0);
      }, TRANSITION_DELAY);
      return () => clearTimeout(delayTimer);
    }

    const timer = setTimeout(() => {
      setVisibleChars((prev) => prev - 1);
    }, CHAR_HIDE_INTERVAL);

    return () => clearTimeout(timer);
  }, [animationPhase, visibleChars]);

  // Handle showing animation (left to right reveal of placeholder)
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
      setVisibleChars((prev) => prev + 1);
    }, CHAR_SHOW_INTERVAL);

    return () => clearTimeout(timer);
  }, [animationPhase, visibleChars, placeholder.length]);

  // Render based on current state
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
      onSessionPrev={onSessionPrev}
      onSessionNext={onSessionNext}
      maxVisibleLines={maxVisibleLines}
      isActive={isActive}
    />
  );
};

export default InputTransition;
