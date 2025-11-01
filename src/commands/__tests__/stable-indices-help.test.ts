/**
 * Feature: spec/features/implement-stable-indices-with-soft-delete.feature
 *
 * Tests for Rules 14-17: Help files and documentation
 */

import { describe, it, expect } from 'vitest';
import { existsSync } from 'fs';
import { join } from 'path';

describe('Feature: Implement Stable Indices with Soft Delete - Help Files', () => {
  describe('Rule 14: Create restore command help files', () => {
    it('should have restore-rule-help.ts', () => {
      const helpFile = join(__dirname, '..', 'restore-rule-help.ts');
      expect(existsSync(helpFile)).toBe(true);
    });

    it('should have restore-example-help.ts', () => {
      const helpFile = join(__dirname, '..', 'restore-example-help.ts');
      expect(existsSync(helpFile)).toBe(true);
    });

    it('should have restore-question-help.ts', () => {
      const helpFile = join(__dirname, '..', 'restore-question-help.ts');
      expect(existsSync(helpFile)).toBe(true);
    });

    it('should have restore-architecture-note-help.ts', () => {
      const helpFile = join(
        __dirname,
        '..',
        'restore-architecture-note-help.ts'
      );
      expect(existsSync(helpFile)).toBe(true);
    });

    it('restore-rule-help.ts should export CommandHelpConfig', async () => {
      const config = await import('../restore-rule-help');
      expect(config.default).toBeDefined();
      expect(config.default.name).toBe('restore-rule');
      expect(config.default.description).toBeDefined();
      expect(config.default.usage).toBeDefined();
      expect(config.default.examples).toBeDefined();
      expect(config.default.relatedCommands).toBeDefined();
    });
  });

  describe('Rule 15: Create compact-work-unit-help.ts', () => {
    it('should have compact-work-unit-help.ts', () => {
      const helpFile = join(__dirname, '..', 'compact-work-unit-help.ts');
      expect(existsSync(helpFile)).toBe(true);
    });

    it('compact-work-unit-help.ts should export CommandHelpConfig', async () => {
      const config = await import('../compact-work-unit-help');
      expect(config.default).toBeDefined();
      expect(config.default.name).toBe('compact-work-unit');
      expect(config.default.description).toBeDefined();
      expect(config.default.usage).toBeDefined();
      expect(config.default.examples).toBeDefined();
      expect(config.default.relatedCommands).toBeDefined();
    });
  });

  describe('Rule 29: Create show-deleted-help.ts', () => {
    it('should have show-deleted-help.ts', () => {
      const helpFile = join(__dirname, '..', 'show-deleted-help.ts');
      expect(existsSync(helpFile)).toBe(true);
    });

    it('show-deleted-help.ts should export CommandHelpConfig', async () => {
      const config = await import('../show-deleted-help');
      expect(config.default).toBeDefined();
      expect(config.default.name).toBe('show-deleted');
      expect(config.default.description).toBeDefined();
      expect(config.default.usage).toBeDefined();
      expect(config.default.examples).toBeDefined();
      expect(config.default.relatedCommands).toBeDefined();
    });
  });
});
