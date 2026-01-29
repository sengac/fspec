/**
 * Feature: Exit confirmation dialog for fspec
 * 
 * Tests the exit confirmation dialog that appears when user presses ESC
 * from the main BoardView, preventing accidental exits.
 */

import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { setupFullTest, type FullTestSetup } from '../../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../../test-helpers/test-file-operations';

// Import the component
import { BoardView } from '../BoardView';

describe('Feature: Exit confirmation dialog for fspec', () => {
  let setup: FullTestSetup;

  beforeEach(async () => {
    setup = await setupFullTest('exit-confirmation');

    // Create minimal foundation.json (override default)
    await writeJsonTestFile(setup.foundationFile, {
      projectName: "Exit Test",
      description: "Testing exit confirmation"
    });

    // Create work-units.json (override default)
    await writeJsonTestFile(setup.workUnitsFile, {
      workUnits: {
        "TEST-001": {
          id: "TEST-001",
          title: "Test Work Unit",
          status: "backlog",
          type: "task",
          estimate: 2
        }
      },
      states: {
        backlog: ["TEST-001"],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: []
      }
    });
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: ESC key shows exit confirmation dialog', () => {
    it('should show confirmation dialog when ESC is pressed from main board view', async () => {
      const onExit = vi.fn();

      const { lastFrame, stdin } = render(
        <BoardView onExit={onExit} cwd={setup.testDir} />
      );

      // Wait for component to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given I am on the main board view
      expect(lastFrame()).toContain('BACKLOG');
      expect(lastFrame()).toContain('TEST-001');

      // @step When I press ESC
      stdin.write('\x1B'); // ESC key

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the exit confirmation dialog should appear
      expect(lastFrame()).toContain('Exit fspec?');
      expect(lastFrame()).toContain('Are you sure you want to exit?');

      // @step And the onExit should not be called yet
      expect(onExit).not.toHaveBeenCalled();
    });

    it('should exit when Yes is selected and Enter is pressed in confirmation dialog', async () => {
      const onExit = vi.fn();

      const { lastFrame, stdin } = render(
        <BoardView onExit={onExit} cwd={setup.testDir} />
      );

      // Wait for component to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given I have pressed ESC and the confirmation dialog is showing
      stdin.write('\x1B'); // ESC key
      await new Promise(resolve => setTimeout(resolve, 50));
      expect(lastFrame()).toContain('Exit fspec?');

      // @step When I navigate to Yes and press Enter
      // The visual mode defaults to 'no', so we need to press Left arrow to get to 'yes'
      stdin.write('\x1B[D'); // Left arrow to navigate to Yes
      await new Promise(resolve => setTimeout(resolve, 50));
      stdin.write('\r'); // Enter to select Yes
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then onExit should be called
      expect(onExit).toHaveBeenCalled();
    });

    it('should cancel exit when No is selected and Enter is pressed in confirmation dialog', async () => {
      const onExit = vi.fn();

      const { lastFrame, stdin } = render(
        <BoardView onExit={onExit} cwd={setup.testDir} />
      );

      // Wait for component to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given I have pressed ESC and the confirmation dialog is showing
      stdin.write('\x1B'); // ESC key
      await new Promise(resolve => setTimeout(resolve, 50));
      expect(lastFrame()).toContain('Exit fspec?');

      // @step When I select No and press Enter
      // The visual mode defaults to 'yes', so we need to navigate to 'no'
      stdin.write('\x1B[C'); // Right arrow to select No
      await new Promise(resolve => setTimeout(resolve, 50));
      stdin.write('\r'); // Enter to select No
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the dialog should close and return to the board
      expect(lastFrame()).not.toContain('Exit fspec?');
      expect(lastFrame()).toContain('BACKLOG');

      // @step And onExit should not be called
      expect(onExit).not.toHaveBeenCalled();
    });

    it('should cancel exit when ESC is pressed in confirmation dialog', async () => {
      const onExit = vi.fn();

      const { lastFrame, stdin } = render(
        <BoardView onExit={onExit} cwd={setup.testDir} />
      );

      // Wait for component to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given I have pressed ESC and the confirmation dialog is showing
      stdin.write('\x1B'); // ESC key
      await new Promise(resolve => setTimeout(resolve, 50));
      expect(lastFrame()).toContain('Exit fspec?');

      // @step When I press ESC again
      stdin.write('\x1B'); // ESC key again
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the dialog should close and return to the board
      expect(lastFrame()).not.toContain('Exit fspec?');
      expect(lastFrame()).toContain('BACKLOG');

      // @step And onExit should not be called
      expect(onExit).not.toHaveBeenCalled();
    });
  });
});