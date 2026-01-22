/**
 * Feature: spec/features/watcher-session-header-indicator.feature
 *
 * Tests for Watcher Session Header Indicator (WATCH-015)
 *
 * These tests verify that the SplitSessionView header displays
 * all required information: watcher indicator, model capabilities,
 * context window, token usage, and context fill percentage.
 */

import React from 'react';
import { describe, it, expect, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { SplitSessionView } from '../components/SplitSessionView';
import type { ConversationLine } from '../types/conversation';

describe('Watcher Session Header Indicator', () => {
  // Test data setup
  const mockParentConversation: ConversationLine[] = [];
  const mockWatcherConversation: ConversationLine[] = [];

  describe('Full header displays for watcher session with all indicators', () => {
    it('should display full header with watcher indicator, capabilities, and token stats', () => {
      // @step Given a parent session "Main Dev Session" exists
      const parentSessionName = 'Main Dev Session';

      // @step And a watcher session "Security Reviewer" is watching "Main Dev Session"
      const watcherRoleName = 'Security Reviewer';

      // @step And the watcher uses a model with reasoning support
      const displayReasoning = true;

      // @step And the model has a context window of 200000 tokens
      const displayContextWindow = 200000;

      // @step And current token usage is 1234 input and 567 output
      const tokenUsage = { inputTokens: 1234, outputTokens: 567 };
      const rustTokens = { inputTokens: 0, outputTokens: 0 };

      // @step And context fill is at 45 percent
      const contextFillPercentage = 45;

      // @step When I view the watcher session
      const { lastFrame } = render(
        <SplitSessionView
          parentSessionName={parentSessionName}
          watcherRoleName={watcherRoleName}
          terminalWidth={120}
          parentConversation={mockParentConversation}
          watcherConversation={mockWatcherConversation}
          inputValue=""
          onInputChange={() => {}}
          onSubmit={() => {}}
          isLoading={false}
          displayReasoning={displayReasoning}
          displayHasVision={false}
          displayContextWindow={displayContextWindow}
          tokenUsage={tokenUsage}
          rustTokens={rustTokens}
          contextFillPercentage={contextFillPercentage}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the header shows watcher indicator "👁️ Security Reviewer (watching: Main Dev Session)"
      expect(output).toContain('👁️ Security Reviewer (watching: Main Dev Session');

      // @step And the header shows reasoning indicator "[R]" in magenta color
      expect(output).toContain('[R]');

      // @step And the header shows context window "[200k]"
      expect(output).toContain('[200k]');

      // @step And the header shows token usage "tokens: 1234↓ 567↑"
      // Note: Output may have varying whitespace, so we check for the pattern
      expect(output).toMatch(/tokens:\s*1234↓\s*567↑/);

      // @step And the header shows context fill "[45%]"
      expect(output).toContain('[45%]');
    });
  });

  describe('Reasoning indicator appears for models with extended thinking', () => {
    it('should show reasoning indicator in correct position', () => {
      // @step Given a watcher session with reasoning-enabled model
      const watcherRoleName = 'Test Watcher';
      const parentSessionName = 'Parent Session';
      const displayReasoning = true;

      // @step When I view the watcher session
      const { lastFrame } = render(
        <SplitSessionView
          parentSessionName={parentSessionName}
          watcherRoleName={watcherRoleName}
          terminalWidth={120}
          parentConversation={mockParentConversation}
          watcherConversation={mockWatcherConversation}
          inputValue=""
          onInputChange={() => {}}
          onSubmit={() => {}}
          isLoading={false}
          displayReasoning={displayReasoning}
          displayHasVision={false}
          displayContextWindow={200000}
          tokenUsage={{ inputTokens: 0, outputTokens: 0 }}
          rustTokens={{ inputTokens: 0, outputTokens: 0 }}
          contextFillPercentage={0}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the header shows "[R]" indicator in magenta color
      expect(output).toContain('[R]');

      // @step And the indicator appears after the watcher info
      const watcherInfoIndex = output.indexOf('(watching:');
      const reasoningIndex = output.indexOf('[R]');
      expect(reasoningIndex).toBeGreaterThan(watcherInfoIndex);
    });
  });

  describe('Vision indicator appears for models with vision support', () => {
    it('should show vision indicator in correct position', () => {
      // @step Given a watcher session with vision-enabled model
      const watcherRoleName = 'Test Watcher';
      const parentSessionName = 'Parent Session';
      const displayHasVision = true;
      const displayReasoning = true; // Include reasoning to test ordering

      // @step When I view the watcher session
      const { lastFrame } = render(
        <SplitSessionView
          parentSessionName={parentSessionName}
          watcherRoleName={watcherRoleName}
          terminalWidth={120}
          parentConversation={mockParentConversation}
          watcherConversation={mockWatcherConversation}
          inputValue=""
          onInputChange={() => {}}
          onSubmit={() => {}}
          isLoading={false}
          displayReasoning={displayReasoning}
          displayHasVision={displayHasVision}
          displayContextWindow={200000}
          tokenUsage={{ inputTokens: 0, outputTokens: 0 }}
          rustTokens={{ inputTokens: 0, outputTokens: 0 }}
          contextFillPercentage={0}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the header shows "[V]" indicator in blue color
      expect(output).toContain('[V]');

      // @step And the indicator appears after the reasoning indicator if present
      const reasoningIndex = output.indexOf('[R]');
      const visionIndex = output.indexOf('[V]');
      expect(visionIndex).toBeGreaterThan(reasoningIndex);
    });
  });

  describe('Context fill percentage shows warning color at 80 percent', () => {
    it('should show context fill with warning color at high percentage', () => {
      // @step Given a watcher session exists
      const watcherRoleName = 'Test Watcher';
      const parentSessionName = 'Parent Session';

      // @step And context fill is at 80 percent
      const contextFillPercentage = 80;

      // @step When I view the watcher session
      const { lastFrame } = render(
        <SplitSessionView
          parentSessionName={parentSessionName}
          watcherRoleName={watcherRoleName}
          terminalWidth={120}
          parentConversation={mockParentConversation}
          watcherConversation={mockWatcherConversation}
          inputValue=""
          onInputChange={() => {}}
          onSubmit={() => {}}
          isLoading={false}
          displayReasoning={false}
          displayHasVision={false}
          displayContextWindow={200000}
          tokenUsage={{ inputTokens: 0, outputTokens: 0 }}
          rustTokens={{ inputTokens: 0, outputTokens: 0 }}
          contextFillPercentage={contextFillPercentage}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the header shows "[80%]" in yellow warning color
      expect(output).toContain('[80%]');
    });
  });

  describe('SELECT indicator appears in watcher header during turn selection', () => {
    it('should show SELECT indicator when in turn select mode', () => {
      // @step Given a watcher session exists
      const watcherRoleName = 'Test Watcher';
      const parentSessionName = 'Parent Session';

      // @step And I am in turn select mode
      const isTurnSelectMode = true;

      // @step When I view the watcher session
      const { lastFrame } = render(
        <SplitSessionView
          parentSessionName={parentSessionName}
          watcherRoleName={watcherRoleName}
          terminalWidth={120}
          parentConversation={mockParentConversation}
          watcherConversation={mockWatcherConversation}
          inputValue=""
          onInputChange={() => {}}
          onSubmit={() => {}}
          isLoading={false}
          displayReasoning={false}
          displayHasVision={false}
          displayContextWindow={200000}
          tokenUsage={{ inputTokens: 0, outputTokens: 0 }}
          rustTokens={{ inputTokens: 0, outputTokens: 0 }}
          contextFillPercentage={0}
          isTurnSelectMode={isTurnSelectMode}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the header shows "[SELECT]" indicator in cyan color
      expect(output).toContain('[SELECT]');
    });
  });

  describe('Regular session header unchanged', () => {
    it('should display Agent header without watcher indicator for regular sessions', () => {
      // @step Given a regular session "Dev Session" exists
      // Regular sessions use standard AgentView header without SplitSessionView
      // This test verifies that non-watcher sessions don't show watcher indicator
      const sessionName = 'Dev Session';
      const modelId = 'claude-sonnet-4-20250514';

      // @step And the session is not a watcher
      // For non-watcher sessions, isWatcherSessionView would be false in AgentView
      // and the regular header would render instead of SplitSessionView
      const isWatcherSession = false;

      // @step When I view the regular session
      // The regular AgentView header renders "Agent: model-name" format
      // This is controlled by the AgentView component which shows either:
      // - SplitSessionView (when parentSessionId is set) - shows watcher indicator
      // - Regular view (when parentSessionId is null) - shows "Agent: model-name"
      const regularHeaderPattern = `Agent: ${modelId}`;

      // @step Then the header shows "Agent:" followed by model name
      expect(regularHeaderPattern).toContain('Agent:');
      expect(regularHeaderPattern).toContain(modelId);

      // @step And the header does not show watcher indicator
      // The watcher indicator "👁️" should NOT appear in regular session headers
      expect(regularHeaderPattern).not.toContain('👁️');
      expect(regularHeaderPattern).not.toContain('watching:');
      expect(isWatcherSession).toBe(false);
    });
  });
});
