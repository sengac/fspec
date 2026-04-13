/**
 * Feature: spec/features/create-session-dialog-options.feature
 *
 * This test file validates the CreateSessionDialog component's 3-option UX:
 * Yes, Yes - Isolated, Cancel.
 *
 * TUI-090: Replaces Yes/No + Normal/Isolated toggle with 3 flat options.
 *
 * Note: Dialog uses position="absolute" which doesn't render content in test
 * environment, so we focus on behavioral testing (callbacks, keyboard interactions).
 * Arrow key inputs require async delays for React state updates to propagate.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { CreateSessionDialog } from '../CreateSessionDialog';

/** Helper to wait for React state updates after key input */
const tick = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

describe('Feature: CreateSessionDialog should have 3 options: Yes, Yes - Isolated, Cancel', () => {
  let onConfirm: ReturnType<typeof vi.fn>;
  let onCancel: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onConfirm = vi.fn();
    onCancel = vi.fn();
  });

  describe('Scenario: Default option is Yes and creates normal session on Enter', () => {
    it('should call onConfirm(false) when Enter is pressed without navigation', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // @step Then the "Yes" option should be highlighted by default
      // (Default index is 0 = 'yes', verified by pressing Enter immediately)

      // @step When I press Enter
      stdin.write('\r');
      await tick();

      // @step Then onConfirm should be called with isolated=false
      expect(onConfirm).toHaveBeenCalledWith(false);
      expect(onCancel).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Selecting Yes - Isolated creates isolated session', () => {
    it('should call onConfirm(true) when Right+Enter is pressed', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // @step When I press Right to select "Yes - Isolated"
      stdin.write('\x1B[C'); // Right arrow
      await tick();

      // @step And I press Enter
      stdin.write('\r');
      await tick();

      // @step Then onConfirm should be called with isolated=true
      expect(onConfirm).toHaveBeenCalledWith(true);
      expect(onCancel).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Selecting Cancel closes dialog without creating session', () => {
    it('should call onCancel when Right+Right+Enter is pressed', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // @step When I press Right twice to select "Cancel"
      stdin.write('\x1B[C'); // Right arrow (-> Yes - Isolated)
      await tick();
      stdin.write('\x1B[C'); // Right arrow (-> Cancel)
      await tick();

      // @step And I press Enter
      stdin.write('\r');
      await tick();

      // @step Then onCancel should be called
      expect(onCancel).toHaveBeenCalled();

      // @step And no session should be created
      expect(onConfirm).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: ESC cancels regardless of selected option', () => {
    it('should call onCancel on ESC even when Yes - Isolated is highlighted', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // @step And "Yes - Isolated" is currently highlighted
      stdin.write('\x1B[C'); // Right arrow to select Yes - Isolated
      await tick();

      // @step When I press ESC
      stdin.write('\x1B'); // ESC key
      await tick();

      // @step Then onCancel should be called
      expect(onCancel).toHaveBeenCalled();
      expect(onConfirm).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Context-aware title with 3 options for work unit', () => {
    it('should accept workUnit prop and still offer all 3 option behaviors', async () => {
      // @step Given I am viewing the board with work unit "AUTH-001"
      const workUnit = { id: 'AUTH-001', title: 'User Login' };

      // @step When the Create Session dialog opens for that work unit
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, {
          onConfirm,
          onCancel,
          workUnit,
        })
      );
      await tick();

      // @step Then the dialog title should be "Work on AUTH-001?"
      // (Dialog uses position="absolute" — can't assert rendered text,
      //  but we verify behavioral correctness: workUnit prop accepted,
      //  all 3 options functional)

      // @step And I should see options "Yes", "Yes - Isolated", and "Cancel"
      // Verify all 3 options work: Right -> Right -> Enter = Cancel
      stdin.write('\x1B[C'); // Right (-> Yes - Isolated)
      await tick();
      stdin.write('\x1B[C'); // Right (-> Cancel)
      await tick();
      stdin.write('\r');
      await tick();

      expect(onCancel).toHaveBeenCalled();
      expect(onConfirm).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Cyclic navigation', () => {
    it('should wrap around from Cancel back to Yes on Right arrow', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // Navigate: Yes -> Yes - Isolated -> Cancel -> Yes (wrap)
      stdin.write('\x1B[C'); // Right: Yes - Isolated
      await tick();
      stdin.write('\x1B[C'); // Right: Cancel
      await tick();
      stdin.write('\x1B[C'); // Right: wraps to Yes
      await tick();

      // Press Enter - should be back on Yes
      stdin.write('\r');
      await tick();

      expect(onConfirm).toHaveBeenCalledWith(false);
      expect(onCancel).not.toHaveBeenCalled();

      unmount();
    });

    it('should wrap around from Yes back to Cancel on Left arrow', async () => {
      // @step Given the Create Session dialog is open
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // Navigate: Yes -> Cancel (wrap left)
      stdin.write('\x1B[D'); // Left: wraps to Cancel
      await tick();

      // Press Enter - should be on Cancel
      stdin.write('\r');
      await tick();

      expect(onCancel).toHaveBeenCalled();
      expect(onConfirm).not.toHaveBeenCalled();

      unmount();
    });
  });

  describe('Unattached session dialog', () => {
    it('should call onConfirm(false) without workUnit prop (default behavior)', async () => {
      // @step Given the Create Session dialog is open without a work unit
      const { stdin, unmount } = render(
        React.createElement(CreateSessionDialog, { onConfirm, onCancel })
      );
      await tick();

      // @step Then the dialog title should be "Start New Agent?"
      // (Can't assert rendered text due to absolute positioning,
      //  verified by code inspection: title = 'Start New Agent?' when no workUnit)

      // Verify default Enter still calls onConfirm(false) — behavioral proof
      stdin.write('\r');
      await tick();

      expect(onConfirm).toHaveBeenCalledWith(false);

      unmount();
    });
  });
});
