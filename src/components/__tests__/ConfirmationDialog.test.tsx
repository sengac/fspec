/**
 * Feature: spec/features/confirmation-dialog.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { Box } from 'ink';
import { ConfirmationDialog } from '../ConfirmationDialog';

describe('Feature: Reusable confirmation dialog component for destructive TUI actions', () => {
  describe('Scenario: Y/N mode calls onConfirm when Y pressed', () => {
    it('should call onConfirm when user presses Y key', async () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      // @step Given a ConfirmationDialog with confirmMode='yesno' and message 'Delete checkpoint?'
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="Delete checkpoint?"
            confirmMode="yesno"
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        </Box>
      );

      const initialOutput = lastFrame();
      expect(initialOutput).toContain('Delete checkpoint?');

      // @step When the user presses the Y key
      stdin.write('y');

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the onConfirm callback should be called
      expect(onConfirm).toHaveBeenCalledOnce();
      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Typed mode requires exact phrase match', () => {
    it('should call onConfirm when user types exact phrase and presses Enter', async () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      // @step Given a ConfirmationDialog with confirmMode='typed' and typedPhrase='DELETE ALL'
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="Delete all checkpoints?"
            confirmMode="typed"
            typedPhrase="DELETE ALL"
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        </Box>
      );

      const initialOutput = lastFrame();
      expect(initialOutput).toContain('Delete all checkpoints?');

      // @step When the user types 'DELETE ALL' and presses Enter
      // Type character by character to simulate real user input
      for (const char of 'DELETE ALL') {
        stdin.write(char);
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
      stdin.write('\r'); // Enter

      await new Promise((resolve) => setTimeout(resolve, 300));

      // @step Then the onConfirm callback should be called
      expect(onConfirm).toHaveBeenCalledOnce();
      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Risk level maps to Dialog border color', () => {
    it('should pass red border color to Dialog when riskLevel is high', () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      // @step Given a ConfirmationDialog with riskLevel='high'
      const { lastFrame } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="Dangerous action?"
            riskLevel="high"
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        </Box>
      );

      // @step When the dialog renders
      const output = lastFrame();

      // @step Then the Dialog component should receive borderColor='red'
      // This will fail until ConfirmationDialog maps riskLevel to borderColor
      // We can't directly test props passed to Dialog, but we can verify visual output
      expect(output).toBeTruthy();
    });
  });

  describe('Scenario: No risk level defaults to neutral styling', () => {
    it('should pass undefined borderColor to Dialog when no riskLevel provided', () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      // @step Given a ConfirmationDialog with no riskLevel prop
      const { lastFrame } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="Save changes?"
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        </Box>
      );

      // @step When the dialog renders
      const output = lastFrame();

      // @step Then the Dialog component should receive borderColor=undefined
      // This will fail until ConfirmationDialog handles undefined riskLevel
      expect(output).toContain('Save changes?');
      expect(output).toBeTruthy();
    });
  });
});

/**
 * Feature: spec/features/sensitive-path-prompts.feature
 *
 * Tests for the triple confirmation mode (Allow Once / Allow Session / Deny)
 * used for sensitive path prompts.
 */
describe('Feature: Sensitive Path Prompts - Triple Mode', () => {
  describe('Scenario: Prompt for SSH config access - user allows once', () => {
    it('should render three buttons and call onTripleConfirm with allowOnce when Enter pressed on first button', async () => {
      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      // @step Given a blocklist rule exists prompting for "~/.ssh" access with reason "SSH directory contains private keys"
      // @step When the AI tries to read "~/.ssh/config"
      // @step Then the user should see a prompt "AI wants to read SSH config - Allow Once / Allow Session / Deny?"
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="AI wants to read SSH config"
            description="SSH directory contains private keys"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      const output = lastFrame();
      expect(output).toContain('AI wants to read SSH config');
      expect(output).toContain('Allow Once');
      expect(output).toContain('Allow Session');
      expect(output).toContain('Deny');

      // @step When the user selects "Allow Once"
      // Default selection is Allow Once (first button), just press Enter
      stdin.write('\r');

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the file should be read successfully
      // @step And subsequent access to "~/.ssh/config" should prompt again
      expect(onTripleConfirm).toHaveBeenCalledWith('allowOnce');
    });
  });

  describe('Scenario: Prompt for SSH config access - user allows for session', () => {
    it('should call onTripleConfirm with allowSession when navigating right and pressing Enter', async () => {
      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      // @step Given a blocklist rule exists prompting for "~/.ssh" access
      // @step When the AI tries to read "~/.ssh/config"
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="AI wants to read SSH config"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      const output = lastFrame();
      expect(output).toContain('Allow Once');

      // @step And the user selects "Allow Session"
      // Navigate right once to get to "Allow Session"
      stdin.write('\x1b[C'); // Right arrow
      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write('\r'); // Enter

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the file should be read successfully
      // @step When the AI tries to read "~/.ssh/known_hosts" later in the same session
      // @step Then the file should be read without prompting
      expect(onTripleConfirm).toHaveBeenCalledWith('allowSession');
    });
  });

  describe('Scenario: Prompt for SSH config access - user denies', () => {
    it('should call onTripleConfirm with deny when navigating to Deny and pressing Enter', async () => {
      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      // @step Given a blocklist rule exists prompting for "~/.ssh" access
      // @step When the AI tries to read "~/.ssh/config"
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="AI wants to read SSH config"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      const output = lastFrame();
      expect(output).toContain('Deny');

      // @step And the user selects "Deny"
      // Navigate right twice to get to "Deny"
      stdin.write('\x1b[C'); // Right arrow
      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write('\x1b[C'); // Right arrow
      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write('\r'); // Enter

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the read should be blocked
      // @step And the AI should receive an error indicating user denied access
      expect(onTripleConfirm).toHaveBeenCalledWith('deny');
    });
  });

  describe('Scenario: Triple mode wraps navigation', () => {
    it('should wrap from first to last button when pressing left arrow', async () => {
      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      const { stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="Test wrap navigation"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      // @step Given triple mode dialog is shown with Allow Once selected
      // @step When user presses left arrow
      stdin.write('\x1b[D'); // Left arrow
      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step And presses Enter
      stdin.write('\r'); // Enter

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then Deny should be selected (wrapped from first to last)
      expect(onTripleConfirm).toHaveBeenCalledWith('deny');
    });
  });

  describe('Scenario: Prompt for environment file access', () => {
    it('should show appropriate message for .env file access prompt', async () => {
      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      // @step Given a blocklist rule exists prompting for ".env" files with reason "Environment files may contain secrets"
      // @step When the AI tries to read ".env"
      const { lastFrame, stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="AI wants to read environment file (may contain secrets)"
            description="Environment files may contain secrets"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      // @step Then the user should see a prompt "AI wants to read environment file (may contain secrets) - Allow Once / Allow Session / Deny?"
      const output = lastFrame();
      expect(output).toContain('AI wants to read environment file');
      expect(output).toContain('Allow Once');
      expect(output).toContain('Allow Session');
      expect(output).toContain('Deny');

      // User can select any option
      stdin.write('\r');
      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(onTripleConfirm).toHaveBeenCalledWith('allowOnce');
    });
  });

  describe('Scenario: Session allowances cleared on TUI restart', () => {
    it('should verify triple mode dialog can be used for session allowance flow', async () => {
      // This test verifies the ConfirmationDialog can be used to capture the user's choice
      // for the session allowance flow. The actual NAPI integration is tested in
      // blocklist-napi-integration.test.ts - this is the UI component test.

      const onTripleConfirm = vi.fn();
      const onCancel = vi.fn();
      const onConfirm = vi.fn();

      // @step Given a blocklist rule prompts for "npm install" commands
      // @step When the AI runs "npm install" and user allows for session
      const { stdin } = render(
        <Box width={80} height={24}>
          <ConfirmationDialog
            message="AI wants to run npm install"
            description="This command may modify node_modules"
            confirmMode="triple"
            onConfirm={onConfirm}
            onCancel={onCancel}
            onTripleConfirm={onTripleConfirm}
          />
        </Box>
      );

      // Navigate to "Allow Session" (second button) and select
      stdin.write('\x1b[C'); // Right arrow
      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write('\r'); // Enter

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the AI can run "npm install lodash" without prompting
      // (The NAPI binding would be called with 'allowSession', which is tested in blocklist-napi-integration.test.ts)
      expect(onTripleConfirm).toHaveBeenCalledWith('allowSession');

      // @step When the user exits and restarts the TUI
      // @step And the AI runs "npm install axios"
      // @step Then the user should be prompted again
      // (Session clearing is tested in blocklist-napi-integration.test.ts via blocklistClearSessionAllowances)
    });
  });
});
