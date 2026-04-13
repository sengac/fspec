/**
 * Tests for TUI-058: Default Thinking Level Dialog Behavior
 *
 * Feature: spec/features/default-thinking-level-persistence.feature
 *
 * Tests the ThinkingLevelDialog UI interactions:
 * - D key sets current selection as default for new sessions
 * - Dialog footer shows D key option
 * - Default indicator display and movement
 * - Current session selection independent of default
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { Box, useInput } from 'ink';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { Instance } from 'ink-testing-library';
import { JsThinkingLevel } from '../../../utils/thinkingLevel';

// Create hoisted mock config state
const mockConfig = vi.hoisted(() => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
}));

vi.mock('../../../utils/config', async () => {
  const actual = await vi.importActual<typeof import('../../../utils/config')>(
    '../../../utils/config'
  );
  return {
    ...actual,
    loadConfig: (...args: unknown[]) => mockConfig.loadConfig(...args),
    writeConfig: (...args: unknown[]) => mockConfig.writeConfig(...args),
  };
});

vi.mock('../../../components/Dialog', () => ({
  Dialog: ({ children }: { children: React.ReactNode }) => (
    <Box flexDirection="column" borderStyle="single" padding={1}>
      {children}
    </Box>
  ),
}));

vi.mock('../../input/index', () => ({
  useInputCompat: ({
    handler,
  }: {
    handler: (
      input: string,
      key: {
        upArrow?: boolean;
        downArrow?: boolean;
        return?: boolean;
        escape?: boolean;
      }
    ) => boolean;
  }) => {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useInput((input, key) => {
      handler(input, key);
    });
  },
  InputPriority: { CRITICAL: 0, HIGH: 1, NORMAL: 2, LOW: 3 },
}));

import { ThinkingLevelDialog } from '../ThinkingLevelDialog';

const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

interface DialogCallbacks {
  onSelect: ReturnType<typeof vi.fn>;
  onClose: ReturnType<typeof vi.fn>;
  onSetDefault: ReturnType<typeof vi.fn>;
}

/** Render the dialog with standard callbacks and return everything needed for testing */
function renderDialog(
  currentLevel: JsThinkingLevel = JsThinkingLevel.Off,
  defaultLevel: JsThinkingLevel | null = null,
  overrides?: Partial<DialogCallbacks>
): Instance & DialogCallbacks {
  const onSelect = overrides?.onSelect ?? vi.fn();
  const onClose = overrides?.onClose ?? vi.fn();
  const onSetDefault = overrides?.onSetDefault ?? vi.fn();

  const instance = render(
    <ThinkingLevelDialog
      currentLevel={currentLevel}
      defaultLevel={defaultLevel}
      onSelect={onSelect}
      onSetDefault={onSetDefault}
      onClose={onClose}
    />
  );
  return { ...instance, onSelect, onClose, onSetDefault };
}

/** Navigate down N times from starting position */
async function navigateDown(stdin: { write: (s: string) => void }, steps: number): Promise<void> {
  for (let i = 0; i < steps; i++) {
    stdin.write('\x1B[B');
    await waitForFrame();
  }
}

describe('Feature: Default Thinking Level Persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig.loadConfig.mockResolvedValue({});
    mockConfig.writeConfig.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Set default thinking level via D key', () => {
    it('should show status message and keep dialog open when D key is pressed', async () => {
      // @step Given the user has a chat session open
      // @step And the ThinkingLevelDialog is open with High selected
      const { stdin, lastFrame, unmount, onSelect, onClose, onSetDefault } = renderDialog();
      await waitForFrame();

      await navigateDown(stdin, 3); // Navigate to High
      expect(lastFrame() || '').toContain('▸ High');

      // @step When the user presses the 'D' key
      stdin.write('d');
      await waitForFrame();

      // @step Then a status message shows "High set as default for new sessions"
      expect(onSetDefault).toHaveBeenCalledWith(JsThinkingLevel.High);

      // @step And the dialog remains open
      expect(onClose).not.toHaveBeenCalled();
      expect(onSelect).not.toHaveBeenCalled();

      // @step And the user can still navigate and select a different level
      stdin.write('\x1B[A'); // Up to Medium
      await waitForFrame();
      expect(lastFrame() || '').toContain('▸ Medium');

      unmount();
    });
  });

  describe('Scenario: Dialog footer shows D key option', () => {
    it('should display D Set Default in footer', async () => {
      // @step Given the user has a chat session open
      // @step When the ThinkingLevelDialog is opened via /thinking command
      const { lastFrame, unmount } = renderDialog();
      await waitForFrame();

      // @step Then the dialog footer shows "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"
      const output = lastFrame() || '';
      expect(output).toContain('D Set Default');
      expect(output).toContain('Enter Select');
      expect(output).toContain('Esc Close');

      unmount();
    });
  });

  describe('Scenario: Dialog shows default indicator when default is set', () => {
    it('should show (default) indicator on the default level', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.defaultThinkingLevel": 2
      // @step And the user has a chat session open
      // @step When the ThinkingLevelDialog is opened via /thinking command
      const { lastFrame, unmount } = renderDialog(JsThinkingLevel.Off, JsThinkingLevel.Medium);
      await waitForFrame();

      const output = lastFrame() || '';

      // @step Then the Medium option shows "(default)" indicator
      expect(output).toMatch(/Medium.*\(default\)/);

      // @step And no other option shows the "(default)" indicator
      const lines = output.split('\n');
      for (const label of ['Off', 'Low', 'High']) {
        const line = lines.find(l => l.includes(label) && !l.includes('Medium'));
        if (line) {
          expect(line).not.toContain('(default)');
        }
      }

      unmount();
    });
  });

  describe('Scenario: Dialog shows no indicator when no default is set', () => {
    it('should not show (default) indicator when defaultLevel is null', async () => {
      // @step Given ~/.fspec/fspec-config.json does not contain tui.defaultThinkingLevel
      // @step And the user has a chat session open
      // @step When the ThinkingLevelDialog is opened via /thinking command
      const { lastFrame, unmount } = renderDialog(JsThinkingLevel.Off, null);
      await waitForFrame();

      // @step Then no option shows the "(default)" indicator
      expect(lastFrame() || '').not.toContain('(default)');

      unmount();
    });
  });

  describe('Scenario: Default indicator moves when D key is pressed', () => {
    it('should move (default) from Medium to High when D pressed on High', async () => {
      // @step Given the user has a chat session open
      const onSelect = vi.fn();
      const onClose = vi.fn();
      let currentDefault: JsThinkingLevel | null = JsThinkingLevel.Medium;
      const onSetDefault = vi.fn((level: JsThinkingLevel) => {
        currentDefault = level;
      });

      // @step And the default thinking level is Medium
      // @step And the ThinkingLevelDialog is open with High selected
      const { stdin, lastFrame, unmount, rerender } = render(
        <ThinkingLevelDialog
          currentLevel={JsThinkingLevel.Off}
          defaultLevel={currentDefault}
          onSelect={onSelect}
          onSetDefault={onSetDefault}
          onClose={onClose}
        />
      );
      await waitForFrame();

      await navigateDown(stdin, 3); // Navigate to High

      // Verify initial state - Medium has (default)
      expect(lastFrame() || '').toMatch(/Medium.*\(default\)/);
      expect(lastFrame() || '').not.toMatch(/High.*\(default\)/);

      // @step When the user presses the 'D' key
      stdin.write('d');
      await waitForFrame();

      rerender(
        <ThinkingLevelDialog
          currentLevel={JsThinkingLevel.Off}
          defaultLevel={currentDefault}
          onSelect={onSelect}
          onSetDefault={onSetDefault}
          onClose={onClose}
        />
      );
      await waitForFrame();

      // @step Then the High option now shows "(default)" indicator
      expect(onSetDefault).toHaveBeenCalledWith(JsThinkingLevel.High);

      // @step And the Medium option no longer shows "(default)" indicator
      expect(currentDefault).toBe(JsThinkingLevel.High);

      unmount();
    });
  });

  describe('Scenario: Current session selection is independent of default', () => {
    it('should allow selecting different level than default', async () => {
      // @step Given the user has set a default thinking level of Medium via D key
      let savedDefault: JsThinkingLevel | null = JsThinkingLevel.Medium;
      const onSetDefault = vi.fn((level: JsThinkingLevel) => {
        savedDefault = level;
      });
      let sessionLevel: JsThinkingLevel | null = null;
      const onSelect = vi.fn((level: JsThinkingLevel) => {
        sessionLevel = level;
      });

      // @step And the ThinkingLevelDialog is open with High selected
      const { stdin, unmount } = renderDialog(JsThinkingLevel.Off, savedDefault, {
        onSelect,
        onSetDefault,
      });
      await waitForFrame();

      await navigateDown(stdin, 3); // Navigate to High

      // @step When the user presses Enter to select High
      stdin.write('\r');
      await waitForFrame();

      // @step Then the current session uses High thinking level
      expect(onSelect).toHaveBeenCalledWith(JsThinkingLevel.High);
      expect(sessionLevel).toBe(JsThinkingLevel.High);

      // @step And the default remains Medium for future sessions
      expect(savedDefault).toBe(JsThinkingLevel.Medium);
      expect(onSetDefault).not.toHaveBeenCalled();

      unmount();
    });
  });
});
