/**
 * Test helpers for MultiLineInput component testing
 * Provides reusable utilities following DRY/SOLID principles
 */

import React from 'react';
import { render, type RenderResult } from 'ink-testing-library';
import { vi, type MockedFunction } from 'vitest';
import { MultiLineInput, type MultiLineInputProps } from '../tui/components/MultiLineInput';
import { InputManager } from '../tui/input/InputManager';
import type { CompactionProgress } from '../tui/hooks/useRustSessionState';

/**
 * Default props for MultiLineInput testing
 * Provides sane defaults while allowing overrides
 */
const defaultMultiLineInputProps: Partial<MultiLineInputProps> = {
  value: '',
  onChange: vi.fn(),
  onSubmit: vi.fn(),
  placeholder: 'Type a message...',
  isActive: true,
  maxVisibleLines: 5,
  suppressEnter: false,
  isCompacting: false,
  compactionProgress: null
};

/**
 * Enhanced render result with typed mock functions and utilities
 */
export interface MultiLineInputTestResult extends RenderResult {
  props: MultiLineInputProps;
  mocks: {
    onChange: MockedFunction<(value: string) => void>;
    onSubmit: MockedFunction<() => void>;
    onHistoryPrev?: MockedFunction<() => void>;
    onHistoryNext?: MockedFunction<() => void>;
  };
  utils: {
    typeText: (text: string) => Promise<void>;
    pressKey: (key: string) => Promise<void>;
    waitForRender: (ms?: number) => Promise<void>;
    expectNoTextChange: () => void;
    expectTextChanged: (expectedCallCount?: number) => void;
    expectSubmitted: (expectedCallCount?: number) => void;
    expectNotSubmitted: () => void;
  };
}

/**
 * Renders MultiLineInput with InputManager and provides testing utilities
 * 
 * @param overrides - Props to override defaults
 * @returns Enhanced render result with testing utilities
 */
export function renderMultiLineInput(
  overrides: Partial<MultiLineInputProps> = {}
): MultiLineInputTestResult {
  // Create mocked functions
  const onChange = vi.fn();
  const onSubmit = vi.fn();
  const onHistoryPrev = overrides.onHistoryPrev ? vi.fn() : undefined;
  const onHistoryNext = overrides.onHistoryNext ? vi.fn() : undefined;
  
  // Merge props with defaults
  const props: MultiLineInputProps = {
    ...defaultMultiLineInputProps,
    ...overrides,
    onChange,
    onSubmit,
    onHistoryPrev,
    onHistoryNext,
  } as MultiLineInputProps;
  
  // Render component with InputManager
  const renderResult = render(
    <InputManager>
      <MultiLineInput {...props} />
    </InputManager>
  );
  
  // Create testing utilities
  const utils = {
    typeText: async (text: string): Promise<void> => {
      renderResult.stdin.write(text);
      await new Promise(resolve => setTimeout(resolve, 20));
    },
    
    pressKey: async (key: string): Promise<void> => {
      renderResult.stdin.write(key);
      await new Promise(resolve => setTimeout(resolve, 20));
    },
    
    waitForRender: async (ms = 20): Promise<void> => {
      await new Promise(resolve => setTimeout(resolve, ms));
    },
    
    expectNoTextChange: (): void => {
      expect(onChange).not.toHaveBeenCalled();
    },
    
    expectTextChanged: (expectedCallCount = 1): void => {
      expect(onChange).toHaveBeenCalledTimes(expectedCallCount);
    },
    
    expectSubmitted: (expectedCallCount = 1): void => {
      expect(onSubmit).toHaveBeenCalledTimes(expectedCallCount);
    },
    
    expectNotSubmitted: (): void => {
      expect(onSubmit).not.toHaveBeenCalled();
    }
  };
  
  const mocks = {
    onChange,
    onSubmit,
    onHistoryPrev,
    onHistoryNext
  };
  
  return {
    ...renderResult,
    props,
    mocks,
    utils
  };
}

/**
 * Creates MultiLineInput props for compaction testing scenarios
 * 
 * @param scenario - Type of compaction scenario
 * @param progress - Compaction progress data
 * @param overrides - Additional prop overrides
 * @returns Props configured for compaction testing
 */
export function createCompactionProps(
  scenario: 'idle' | 'compacting' | 'completing',
  progress?: CompactionProgress | null,
  overrides: Partial<MultiLineInputProps> = {}
): Partial<MultiLineInputProps> {
  const baseProps: Partial<MultiLineInputProps> = {
    ...overrides
  };
  
  switch (scenario) {
    case 'idle':
      return {
        ...baseProps,
        isCompacting: false,
        compactionProgress: null,
        suppressEnter: false
      };
      
    case 'compacting':
      return {
        ...baseProps,
        isCompacting: true,
        compactionProgress: progress || null,
        suppressEnter: true
      };
      
    case 'completing':
      return {
        ...baseProps,
        isCompacting: false,
        compactionProgress: null,
        suppressEnter: false
      };
      
    default:
      return baseProps;
  }
}

/**
 * Renders MultiLineInput in compacting state with progress
 * 
 * @param progress - Compaction progress to display
 * @param overrides - Additional prop overrides  
 * @returns Render result configured for compaction testing
 */
export function renderCompactingInput(
  progress: CompactionProgress,
  overrides: Partial<MultiLineInputProps> = {}
): MultiLineInputTestResult {
  const compactionProps = createCompactionProps('compacting', progress, overrides);
  return renderMultiLineInput(compactionProps);
}

/**
 * Renders MultiLineInput in idle state (not compacting)
 * 
 * @param overrides - Prop overrides
 * @returns Render result configured for idle testing
 */
export function renderIdleInput(
  overrides: Partial<MultiLineInputProps> = {}
): MultiLineInputTestResult {
  const idleProps = createCompactionProps('idle', null, overrides);
  return renderMultiLineInput(idleProps);
}

/**
 * Simulates state transition from idle -> compacting -> completing
 * Useful for testing full compaction lifecycle
 * 
 * @param progress - Compaction progress to show during compaction
 * @param overrides - Base prop overrides
 * @returns Object with methods to trigger state transitions
 */
export function createCompactionLifecycleTest(
  progress: CompactionProgress,
  overrides: Partial<MultiLineInputProps> = {}
) {
  let currentRender: MultiLineInputTestResult;
  
  const startIdle = () => {
    currentRender = renderIdleInput(overrides);
    return currentRender;
  };
  
  const startCompacting = () => {
    if (!currentRender) throw new Error('Must call startIdle() first');
    
    const compactingProps = createCompactionProps('compacting', progress, overrides);
    currentRender.rerender(
      <InputManager>
        <MultiLineInput {...currentRender.props} {...compactingProps} />
      </InputManager>
    );
    return currentRender;
  };
  
  const finishCompacting = () => {
    if (!currentRender) throw new Error('Must call startCompacting() first');
    
    const completingProps = createCompactionProps('completing', null, overrides);
    currentRender.rerender(
      <InputManager>
        <MultiLineInput {...currentRender.props} {...completingProps} />
      </InputManager>
    );
    return currentRender;
  };
  
  return {
    startIdle,
    startCompacting, 
    finishCompacting,
    getCurrentRender: () => currentRender
  };
}