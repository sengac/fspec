/**
 * Feature: spec/features/watcher-templates.feature
 * Tests for NotificationDialog component
 *
 * This component shows success/info/warning notifications with auto-dismiss.
 * Pattern follows StatusDialog.test.tsx for consistency.
 *
 * Note: Dialog uses position="absolute" which doesn't render content in test
 * environment, so we focus on behavioral testing (timers, callbacks).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import {
  NotificationDialog,
  NotificationDialogProps,
} from '../NotificationDialog';

describe('Feature: Watcher Template Feedback Dialogs', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('Scenario: Success notification auto-dismisses after 2 seconds', () => {
    it('should auto-close after autoDismissMs', () => {
      // @step Given a watcher was spawned successfully
      const onClose = vi.fn();

      // @step When a success notification shows "Spawned watcher"
      render(
        React.createElement(NotificationDialog, {
          message: 'Spawned watcher "Security Reviewer" from template',
          type: 'success',
          autoDismissMs: 2000,
          onClose,
        })
      );

      // @step Then the dialog should not close immediately
      expect(onClose).not.toHaveBeenCalled();

      // @step Then the dialog should auto-close after 2 seconds
      vi.advanceTimersByTime(2000);
      expect(onClose).toHaveBeenCalledOnce();
    });

    it('should auto-close after custom autoDismissMs (3000ms)', () => {
      const onClose = vi.fn();

      render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          type: 'success',
          autoDismissMs: 3000,
          onClose,
        })
      );

      // Should not close at 2999ms
      vi.advanceTimersByTime(2999);
      expect(onClose).not.toHaveBeenCalled();

      // Should close at 3000ms
      vi.advanceTimersByTime(1);
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  describe('Scenario: Notification can be dismissed early with ESC', () => {
    it('should allow ESC to close notification early', async () => {
      // @step Given a success notification is displayed
      const onClose = vi.fn();
      const { stdin } = render(
        React.createElement(NotificationDialog, {
          message: 'Deleted template "Security Reviewer"',
          type: 'success',
          autoDismissMs: 2000,
          onClose,
        })
      );

      // @step When I press ESC before 2 seconds elapse
      stdin.write('\x1B'); // ESC key

      // Wait for input to be processed
      await vi.advanceTimersByTimeAsync(0);
      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(0);

      // @step Then the dialog should close immediately
      expect(onClose).toHaveBeenCalled();
    });

    it('should close via ESC for all notification types', async () => {
      // Test success type
      const onCloseSuccess = vi.fn();
      const { stdin: stdinSuccess } = render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          type: 'success',
          onClose: onCloseSuccess,
        } as NotificationDialogProps)
      );
      stdinSuccess.write('\x1B');
      await vi.advanceTimersByTimeAsync(0);
      await Promise.resolve();
      expect(onCloseSuccess).toHaveBeenCalled();

      // Test info type
      const onCloseInfo = vi.fn();
      const { stdin: stdinInfo } = render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          type: 'info',
          onClose: onCloseInfo,
        } as NotificationDialogProps)
      );
      stdinInfo.write('\x1B');
      await vi.advanceTimersByTimeAsync(0);
      await Promise.resolve();
      expect(onCloseInfo).toHaveBeenCalled();

      // Test warning type
      const onCloseWarning = vi.fn();
      const { stdin: stdinWarning } = render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          type: 'warning',
          onClose: onCloseWarning,
        } as NotificationDialogProps)
      );
      stdinWarning.write('\x1B');
      await vi.advanceTimersByTimeAsync(0);
      await Promise.resolve();
      expect(onCloseWarning).toHaveBeenCalled();
    });
  });

  describe('Scenario: Notification with disabled auto-dismiss', () => {
    it('should not auto-close when autoDismissMs is 0', () => {
      // @step Given a notification with autoDismissMs=0
      const onClose = vi.fn();
      render(
        React.createElement(NotificationDialog, {
          message: 'Manual dismiss required',
          type: 'info',
          autoDismissMs: 0,
          onClose,
        })
      );

      // @step When 10 seconds pass
      vi.advanceTimersByTime(10000);

      // @step Then the dialog should still be open (not auto-closed)
      expect(onClose).not.toHaveBeenCalled();
    });

    it('should still allow ESC to dismiss when autoDismissMs is 0', async () => {
      const onClose = vi.fn();
      const { stdin } = render(
        React.createElement(NotificationDialog, {
          message: 'Manual dismiss required',
          type: 'info',
          autoDismissMs: 0,
          onClose,
        })
      );

      // ESC should still work
      stdin.write('\x1B');
      await vi.advanceTimersByTimeAsync(0);
      await Promise.resolve();
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe('Scenario: Default props work correctly', () => {
    it('should default to 2000ms auto-dismiss', () => {
      // @step Given a notification without autoDismissMs specified
      const onClose = vi.fn();
      render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          onClose,
        } as NotificationDialogProps)
      );

      // @step When 1999ms pass
      vi.advanceTimersByTime(1999);
      expect(onClose).not.toHaveBeenCalled();

      // @step When 2000ms total pass
      vi.advanceTimersByTime(1);
      expect(onClose).toHaveBeenCalled();
    });

    it('should default to success type', () => {
      // Component renders without error with minimal props
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Test',
            onClose,
          } as NotificationDialogProps)
        );
      }).not.toThrow();
    });
  });

  describe('Scenario: NotificationDialog component renders', () => {
    it('should render without throwing for success type', () => {
      // @step Given a success notification
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Spawned watcher "Security Reviewer" from template',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
      }).not.toThrow();
    });

    it('should render without throwing for info type', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Information message',
            type: 'info',
            autoDismissMs: 2000,
            onClose,
          })
        );
      }).not.toThrow();
    });

    it('should render without throwing for warning type', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Warning message',
            type: 'warning',
            autoDismissMs: 2000,
            onClose,
          })
        );
      }).not.toThrow();
    });
  });

  describe('Scenario: Verify countdown timer decrements', () => {
    it('should call onClose only once even with multiple intervals', () => {
      const onClose = vi.fn();
      render(
        React.createElement(NotificationDialog, {
          message: 'Test',
          type: 'success',
          autoDismissMs: 2000,
          onClose,
        })
      );

      // Advance through the entire duration
      vi.advanceTimersByTime(3000);

      // Should only call onClose once
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: NotificationDialog accepts all watcher operation messages', () => {
    // These verify component accepts the message formats used in AgentView
    // Note: Can't verify rendered text due to Dialog's absolute positioning

    it('should accept spawn success message format', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Spawned watcher "Security Reviewer" from template',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
        vi.advanceTimersByTime(2000);
      }).not.toThrow();
      expect(onClose).toHaveBeenCalled();
    });

    it('should accept switch success message format', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Switched to watcher: Security Reviewer',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
        vi.advanceTimersByTime(2000);
      }).not.toThrow();
      expect(onClose).toHaveBeenCalled();
    });

    it('should accept delete success message format', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Deleted template "Security Reviewer"',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
        vi.advanceTimersByTime(2000);
      }).not.toThrow();
      expect(onClose).toHaveBeenCalled();
    });

    it('should accept kill success message format', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Killed watcher instance',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
        vi.advanceTimersByTime(2000);
      }).not.toThrow();
      expect(onClose).toHaveBeenCalled();
    });

    it('should accept create/update success message format', () => {
      const onClose = vi.fn();
      expect(() => {
        render(
          React.createElement(NotificationDialog, {
            message: 'Created template "New Template"',
            type: 'success',
            autoDismissMs: 2000,
            onClose,
          })
        );
        vi.advanceTimersByTime(2000);
      }).not.toThrow();
      expect(onClose).toHaveBeenCalled();
    });
  });
});
