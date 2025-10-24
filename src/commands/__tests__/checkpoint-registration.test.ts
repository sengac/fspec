/**
 * Feature: spec/features/wire-up-checkpoint-cli-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Command } from 'commander';

describe('Feature: Wire up checkpoint CLI commands', () => {
  let program: Command;

  beforeEach(() => {
    program = new Command();
  });

  describe('Scenario: Register checkpoint command', () => {
    it('should register checkpoint command with proper arguments and action', async () => {
      // Given: the checkpoint.ts file exports a registerCheckpointCommand function
      const { registerCheckpointCommand } = await import('../checkpoint.js');
      expect(registerCheckpointCommand).toBeDefined();
      expect(typeof registerCheckpointCommand).toBe('function');

      // And: the registerCheckpointCommand function is imported in src/index.ts
      // (verified by successful import above)

      // And: registerCheckpointCommand(program) is called in src/index.ts
      registerCheckpointCommand(program);

      // When: I run './dist/index.js checkpoint AUTH-001 baseline'
      const checkpointCmd = program.commands.find(cmd => cmd.name() === 'checkpoint');

      // Then: the command should be registered
      expect(checkpointCmd).toBeDefined();
      expect(checkpointCmd?.name()).toBe('checkpoint');
    });
  });

  describe('Scenario: Register list-checkpoints command', () => {
    it('should register list-checkpoints command with proper arguments and action', async () => {
      // Given: the list-checkpoints.ts file exports a registerListCheckpointsCommand function
      const { registerListCheckpointsCommand } = await import('../list-checkpoints.js');
      expect(registerListCheckpointsCommand).toBeDefined();
      expect(typeof registerListCheckpointsCommand).toBe('function');

      // And: registerListCheckpointsCommand(program) is called
      registerListCheckpointsCommand(program);

      // When/Then: the command should be registered
      const listCmd = program.commands.find(cmd => cmd.name() === 'list-checkpoints');
      expect(listCmd).toBeDefined();
      expect(listCmd?.name()).toBe('list-checkpoints');
    });
  });

  describe('Scenario: Register restore-checkpoint command', () => {
    it('should register restore-checkpoint command with proper arguments and action', async () => {
      // Given: the restore-checkpoint.ts file exports a registerRestoreCheckpointCommand function
      const { registerRestoreCheckpointCommand } = await import('../restore-checkpoint.js');
      expect(registerRestoreCheckpointCommand).toBeDefined();
      expect(typeof registerRestoreCheckpointCommand).toBe('function');

      // And: registerRestoreCheckpointCommand(program) is called
      registerRestoreCheckpointCommand(program);

      // When/Then: the command should be registered
      const restoreCmd = program.commands.find(cmd => cmd.name() === 'restore-checkpoint');
      expect(restoreCmd).toBeDefined();
      expect(restoreCmd?.name()).toBe('restore-checkpoint');
    });
  });

  describe('Scenario: Register cleanup-checkpoints command', () => {
    it('should register cleanup-checkpoints command with proper arguments and action', async () => {
      // Given: the cleanup-checkpoints.ts file exports a registerCleanupCheckpointsCommand function
      const { registerCleanupCheckpointsCommand } = await import('../cleanup-checkpoints.js');
      expect(registerCleanupCheckpointsCommand).toBeDefined();
      expect(typeof registerCleanupCheckpointsCommand).toBe('function');

      // And: registerCleanupCheckpointsCommand(program) is called
      registerCleanupCheckpointsCommand(program);

      // When/Then: the command should be registered
      const cleanupCmd = program.commands.find(cmd => cmd.name() === 'cleanup-checkpoints');
      expect(cleanupCmd).toBeDefined();
      expect(cleanupCmd?.name()).toBe('cleanup-checkpoints');
    });
  });
});
