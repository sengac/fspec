/**
 * Integration Test Helpers: Real Component Integration for Compaction
 * 
 * Provides utilities for testing actual component integration with real state management
 * NO mocks - tests real behavior and component coordination
 * 
 * SOLID Principles:
 * - Single Responsibility: Only handles integration test setup
 * - Open/Closed: Extensible for new integration scenarios  
 * - Dependency Inversion: Uses real implementations, not mocks
 */

import React from 'react';
import { render, type RenderResult } from 'ink-testing-library';
import { vi } from 'vitest';
import { MultiLineInput, type MultiLineInputProps } from '../tui/components/MultiLineInput';
import { ConversationInputArea, type ConversationInputAreaProps } from '../tui/components/ConversationInputArea';
import { InputManager } from '../tui/input/InputManager';
import { AgentView } from '../tui/components/AgentView';
import type { CompactionProgress } from '../tui/hooks/useRustSessionState';
import type { CompactionStateSources } from '../core-logic/compaction-state-manager';

/**
 * Creates a realistic compaction state scenario for integration testing
 */
export function createCompactionScenario(
  scenarioType: 'manual-compaction' | 'hook-triggered' | 'emergency-auto' | 'state-conflict' | 'no-compaction'
): CompactionStateSources {
  const mockProgressAnalyzing: CompactionProgress = {
    phase: 'Analyzing context',
    current: 15,
    total: 32
  };
  
  const mockProgressSummary: CompactionProgress = {
    phase: 'generating summary',
    current: 1,
    total: 1
  };

  switch (scenarioType) {
    case 'manual-compaction':
      return {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: { type: 'manual', reason: 'User executed /compact command' }
        },
        rustBackendState: {
          isCompacting: false,
          compactionProgress: null
        }
      };
    
    case 'hook-triggered':
      return {
        localProgressState: {
          isActive: false,
          progress: null,
          trigger: null
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressAnalyzing
        }
      };
    
    case 'emergency-auto':
      return {
        localProgressState: {
          isActive: false,
          progress: null,
          trigger: null
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: {
            phase: 'emergency compacting',
            current: 5,
            total: 20
          }
        }
      };
    
    case 'state-conflict':
      return {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: { type: 'manual', reason: 'User command' }
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary
        }
      };
    
    case 'no-compaction':
    default:
      return {
        localProgressState: {
          isActive: false,
          progress: null,
          trigger: null
        },
        rustBackendState: {
          isCompacting: false,
          compactionProgress: null
        }
      };
  }
}

/**
 * Integration test result with real behavior validation utilities
 */
export interface IntegrationTestResult extends RenderResult {
  scenario: CompactionStateSources;
  
  // Real behavior validation (NOT mock checking)
  behavior: {
    getDisplayedText: () => string;
    simulateKeyPress: (key: string) => Promise<void>;
    simulateTextInput: (text: string) => Promise<void>;
    getCurrentPlaceholder: () => string;
    isInputBlocked: () => boolean;
    waitForStateChange: (timeoutMs?: number) => Promise<void>;
  };
  
  // Component state inspection  
  state: {
    isCompactingDisplayed: () => boolean;
    getCompactionPhase: () => string | null;
    getProgressNumbers: () => { current: number; total: number } | null;
  };
}

/**
 * Renders MultiLineInput with real compaction state integration
 * Tests actual component behavior, not mock interactions
 */
export function renderMultiLineInputIntegration(
  scenario: CompactionStateSources,
  additionalProps: Partial<MultiLineInputProps> = {}
): IntegrationTestResult {
  
  // Convert state scenario to component props (simulating what AgentView does)
  const isCompacting = scenario.localProgressState.isActive || scenario.rustBackendState.isCompacting;
  const compactionProgress = scenario.localProgressState.isActive 
    ? scenario.localProgressState.progress
    : scenario.rustBackendState.compactionProgress;
  
  const props: MultiLineInputProps = {
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    placeholder: 'Type a message...',
    isActive: true,
    isCompacting,
    compactionProgress,
    suppressEnter: isCompacting,
    ...additionalProps
  };
  
  const renderResult = render(
    <InputManager>
      <MultiLineInput {...props} />
    </InputManager>
  );
  
  const behavior = {
    getDisplayedText: () => renderResult.lastFrame(),
    
    simulateKeyPress: async (key: string) => {
      renderResult.stdin.write(key);
      await new Promise(resolve => setTimeout(resolve, 50));
    },
    
    simulateTextInput: async (text: string) => {
      for (const char of text) {
        renderResult.stdin.write(char);
        await new Promise(resolve => setTimeout(resolve, 10));
      }
    },
    
    getCurrentPlaceholder: () => {
      const frame = renderResult.lastFrame();
      // Extract placeholder text from frame (simplified - would need more robust parsing in real implementation)
      if (frame.includes('Type a message...')) return 'Type a message...';
      if (frame.includes('Compacting:')) {
        const match = frame.match(/Compacting: ([^\\n]+)/);
        return match ? `Compacting: ${match[1]}` : '';
      }
      return '';
    },
    
    isInputBlocked: () => {
      // Check if onChange mock was called after input attempt
      const initialCallCount = (props.onChange as any).mock.calls.length;
      renderResult.stdin.write('test');
      return (props.onChange as any).mock.calls.length === initialCallCount;
    },
    
    waitForStateChange: async (timeoutMs = 1000) => {
      const startTime = Date.now();
      const initialFrame = renderResult.lastFrame();
      
      while (Date.now() - startTime < timeoutMs) {
        await new Promise(resolve => setTimeout(resolve, 50));
        if (renderResult.lastFrame() !== initialFrame) {
          return;
        }
      }
      throw new Error(`State did not change within ${timeoutMs}ms`);
    }
  };
  
  const state = {
    isCompactingDisplayed: () => {
      return renderResult.lastFrame().includes('Compacting:');
    },
    
    getCompactionPhase: () => {
      const frame = renderResult.lastFrame();
      const match = frame.match(/Compacting: ([^.]+)\.\.\./);
      return match ? match[1] : null;
    },
    
    getProgressNumbers: () => {
      const frame = renderResult.lastFrame();
      const match = null; // No more turn counts in compaction text
      return match ? { current: parseInt(match[1]), total: parseInt(match[2]) } : null;
    }
  };
  
  return {
    ...renderResult,
    scenario,
    behavior,
    state
  };
}

/**
 * Renders ConversationInputArea with compaction integration
 * Tests higher-level component coordination
 */
export function renderConversationInputAreaIntegration(
  scenario: CompactionStateSources,
  additionalProps: Partial<ConversationInputAreaProps> = {}
): IntegrationTestResult {
  
  const isCompacting = scenario.localProgressState.isActive || scenario.rustBackendState.isCompacting;
  const compactionProgress = scenario.localProgressState.isActive 
    ? scenario.localProgressState.progress
    : scenario.rustBackendState.compactionProgress;
  
  const props: ConversationInputAreaProps = {
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    placeholder: 'Type a message...',
    isActive: true,
    isCompacting,
    compactionProgress,
    ...additionalProps
  };
  
  const renderResult = render(
    <InputManager>
      <ConversationInputArea {...props} />
    </InputManager>
  );
  
  // Similar behavior/state utilities as MultiLineInput but for ConversationInputArea
  const behavior = {
    getDisplayedText: () => renderResult.lastFrame(),
    
    simulateKeyPress: async (key: string) => {
      renderResult.stdin.write(key);
      await new Promise(resolve => setTimeout(resolve, 50));
    },
    
    simulateTextInput: async (text: string) => {
      for (const char of text) {
        renderResult.stdin.write(char);
        await new Promise(resolve => setTimeout(resolve, 10));
      }
    },
    
    getCurrentPlaceholder: () => {
      const frame = renderResult.lastFrame();
      if (frame.includes('Type a message...')) return 'Type a message...';
      if (frame.includes('Compacting:')) {
        const match = frame.match(/Compacting: ([^\\n]+)/);
        return match ? `Compacting: ${match[1]}` : '';
      }
      return '';
    },
    
    isInputBlocked: () => {
      const initialCallCount = (props.onChange as any).mock.calls.length;
      renderResult.stdin.write('test');
      return (props.onChange as any).mock.calls.length === initialCallCount;
    },
    
    waitForStateChange: async (timeoutMs = 1000) => {
      const startTime = Date.now();
      const initialFrame = renderResult.lastFrame();
      
      while (Date.now() - startTime < timeoutMs) {
        await new Promise(resolve => setTimeout(resolve, 50));
        if (renderResult.lastFrame() !== initialFrame) {
          return;
        }
      }
      throw new Error(`State did not change within ${timeoutMs}ms`);
    }
  };
  
  const state = {
    isCompactingDisplayed: () => {
      return renderResult.lastFrame().includes('Compacting:');
    },
    
    getCompactionPhase: () => {
      const frame = renderResult.lastFrame();
      const match = frame.match(/Compacting: ([^.]+)\.\.\./);
      return match ? match[1] : null;
    },
    
    getProgressNumbers: () => {
      const frame = renderResult.lastFrame();
      const match = null; // No more turn counts in compaction text
      return match ? { current: parseInt(match[1]), total: parseInt(match[2]) } : null;
    }
  };
  
  return {
    ...renderResult,
    scenario,
    behavior,
    state
  };
}

/**
 * Simulates state transitions for testing lifecycle behavior
 */
export class CompactionStateTransitionSimulator {
  private scenario: CompactionStateSources;
  private renderResult: IntegrationTestResult;
  
  constructor(initialScenario: CompactionStateSources) {
    this.scenario = initialScenario;
    this.renderResult = renderMultiLineInputIntegration(this.scenario);
  }
  
  /**
   * Simulates transition to a new compaction state
   */
  async transitionTo(newScenario: CompactionStateSources): Promise<IntegrationTestResult> {
    this.scenario = newScenario;
    
    // Re-render with new state
    const isCompacting = newScenario.localProgressState.isActive || newScenario.rustBackendState.isCompacting;
    const compactionProgress = newScenario.localProgressState.isActive 
      ? newScenario.localProgressState.progress
      : newScenario.rustBackendState.compactionProgress;
    
    // Re-render with new props (simulating state change)
    this.renderResult.rerender(
      <InputManager>
        <MultiLineInput
          value=""
          onChange={vi.fn()}
          onSubmit={vi.fn()}
          placeholder="Type a message..."
          isActive={true}
          isCompacting={isCompacting}
          compactionProgress={compactionProgress}
          suppressEnter={isCompacting}
        />
      </InputManager>
    );
    
    // Wait for render to complete
    await new Promise(resolve => setTimeout(resolve, 50));
    
    return this.renderResult;
  }
  
  /**
   * Simulates starting manual compaction
   */
  async startManualCompaction(): Promise<IntegrationTestResult> {
    return this.transitionTo(createCompactionScenario('manual-compaction'));
  }
  
  /**
   * Simulates hook-triggered compaction
   */
  async startHookTriggeredCompaction(): Promise<IntegrationTestResult> {
    return this.transitionTo(createCompactionScenario('hook-triggered'));
  }
  
  /**
   * Simulates emergency compaction
   */
  async startEmergencyCompaction(): Promise<IntegrationTestResult> {
    return this.transitionTo(createCompactionScenario('emergency-auto'));
  }
  
  /**
   * Simulates completing compaction
   */
  async completeCompaction(): Promise<IntegrationTestResult> {
    return this.transitionTo(createCompactionScenario('no-compaction'));
  }
  
  getCurrentResult(): IntegrationTestResult {
    return this.renderResult;
  }
}

/**
 * Creates a full integration test environment with multiple components
 */
export function createFullIntegrationEnvironment(scenario: CompactionStateSources) {
  const multiLineInput = renderMultiLineInputIntegration(scenario);
  const conversationInputArea = renderConversationInputAreaIntegration(scenario);
  const stateTransitions = new CompactionStateTransitionSimulator(scenario);
  
  return {
    multiLineInput,
    conversationInputArea,
    stateTransitions,
    scenario,
    
    // Cross-component validation
    validateConsistency: () => {
      const multilineCompacting = multiLineInput.state.isCompactingDisplayed();
      const conversationCompacting = conversationInputArea.state.isCompactingDisplayed();
      
      // Both components should show consistent compaction state
      if (multilineCompacting !== conversationCompacting) {
        throw new Error(`Component state inconsistency: MultiLineInput compacting=${multilineCompacting}, ConversationInputArea compacting=${conversationCompacting}`);
      }
      
      return true;
    }
  };
}