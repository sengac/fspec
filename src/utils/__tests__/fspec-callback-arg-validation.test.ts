/**
 * Feature: Fspec callback arg validation and introspection
 *
 * Tests that fspec-callback pre-validates args against Commander.js command
 * definitions and returns structured, AI-friendly error messages.
 */

import { describe, it, expect } from 'vitest';
import { fspecCallback } from '../fspec-callback';

describe('fspec-callback arg validation', () => {
  describe('Scenario: Unknown named option passed to command', () => {
    it('should return structured error with command usage when invalid arg is passed', async () => {
      const result = JSON.parse(
        await fspecCallback(
          'audit-coverage',
          JSON.stringify({ _: ['some-feature'], fix: true }),
          process.cwd()
        )
      );

      expect(result.success).toBe(false);
      expect(result.errorType).toBe('InvalidArgs');
      expect(result.error).toContain('fix');
      expect(result.error).toContain('audit-coverage');
      expect(result.invalidArgs).toEqual(['fix']);
      // Should include introspected command usage in Fspec tool format
      expect(result.commandUsage).toBeDefined();
      expect(result.commandUsage).toContain('audit-coverage');
      expect(result.commandUsage).toContain('Fspec Tool Call');
      // Should NOT contain raw CLI format like "fspec audit-coverage --fix"
      expect(result.commandUsage).not.toContain('fspec audit-coverage');
    });
  });

  describe('Scenario: --help embedded in command string', () => {
    it('should return introspected command usage', async () => {
      const result = JSON.parse(
        await fspecCallback('audit-coverage --help', '{}', process.cwd())
      );

      expect(result.success).toBe(true);
      expect(result.data).toContain('audit-coverage');
      expect(result.data).toContain('Fspec Tool Call');
      expect(result.data).toContain('command: "audit-coverage"');
    });
  });

  describe('Scenario: help command for a command not in manual docs', () => {
    it('should fall back to Commander introspection', async () => {
      const result = JSON.parse(
        await fspecCallback(
          'help',
          JSON.stringify({ command: 'audit-coverage' }),
          process.cwd()
        )
      );

      expect(result.success).toBe(true);
      expect(result.data).toContain('audit-coverage');
      expect(result.data).toContain('Fspec Tool Call');
    });
  });

  describe('Scenario: Valid args should pass through normally', () => {
    it('should not block valid option keys', async () => {
      // list-work-units has --status option
      const result = JSON.parse(
        await fspecCallback(
          'list-work-units',
          JSON.stringify({ status: 'backlog' }),
          process.cwd()
        )
      );

      // Should succeed (or at least not fail with InvalidArgs)
      expect(result.errorType).not.toBe('InvalidArgs');
    });
  });

  describe('Scenario: --help for nonexistent command', () => {
    it('should return command not found', async () => {
      const result = JSON.parse(
        await fspecCallback('nonexistent-command --help', '{}', process.cwd())
      );

      expect(result.success).toBe(false);
      expect(result.errorType).toBe('CommandNotFound');
    });
  });
});
