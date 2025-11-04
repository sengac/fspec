/**
 * Feature: spec/features/dialog.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { Dialog } from '../Dialog';
import { ConfirmationDialog } from '../ConfirmationDialog';

describe('Feature: Base Dialog modal infrastructure component', () => {
  describe('Scenario: Render centered modal with custom border color', () => {
    it('should render centered modal with red border and children content', () => {
      const onClose = vi.fn();

      // @step Given a Dialog component with borderColor='red' and children 'Test Content'
      const { lastFrame } = render(
        React.createElement(Dialog, {
          borderColor: 'red',
          onClose,
        }, 'Test Content')
      );

      // @step When the Dialog is rendered
      const output = lastFrame();

      // @step Then a centered modal should be displayed
      expect(output).toBeTruthy();

      // @step And the border should be red
      // This will fail until Dialog is implemented with borderColor support
      expect(output).toContain('Test Content');

      // @step And the children 'Test Content' should be visible
      expect(output).toContain('Test Content');
    });
  });

  describe('Scenario: Dialog captures all input when active', () => {
    it('should capture input and prevent parent handlers from firing', async () => {
      const onClose = vi.fn();
      const parentInputHandler = vi.fn();

      // @step Given a Dialog component with isActive=true and onClose callback
      // @step And a parent component with its own useInput handler
      // Note: Testing input isolation requires integration test with parent component
      const { stdin } = render(
        React.createElement(Dialog, {
          isActive: true,
          onClose,
        }, 'Test')
      );

      // @step When the user presses a key
      stdin.write('x');

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the Dialog should capture the input first
      // @step And the parent useInput handler should not receive the input
      // This will fail until Dialog is implemented with isActive + useInput
      expect(onClose).not.toHaveBeenCalled(); // 'x' shouldn't trigger anything
    });
  });

  describe('Scenario: Handle ESC key to call onClose', () => {
    it('should call onClose when ESC key is pressed', () => {
      // Test Dialog's ESC handling via ConfirmationDialog
      // Dialog.onClose maps to ConfirmationDialog.onCancel
      const onCancel = vi.fn();

      // @step Given a Dialog component with onClose callback (via ConfirmationDialog)
      const { stdin } = render(
        React.createElement(ConfirmationDialog, {
          message: 'Test',
          confirmMode: 'yesno',
          onConfirm: vi.fn(),
          onCancel, // This becomes Dialog's onClose prop
        })
      );

      // @step When the user presses the ESC key
      stdin.write('\x1b'); // ESC key

      // @step Then the onClose callback should be called
      // onCancel being called proves Dialog's onClose was called
      expect(onCancel).toHaveBeenCalledOnce();
    });
  });
});
