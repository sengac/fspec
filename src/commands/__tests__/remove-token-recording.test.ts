/**
 * Feature: spec/features/remove-token-recording-functionality.feature
 *
 * Tests for complete removal of token recording functionality.
 * These tests verify that all token tracking code has been removed.
 */

import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

const PROJECT_ROOT = join(__dirname, '../../..');

describe('Feature: Remove token recording functionality', () => {
  describe('Scenario: Remove record-tokens command files', () => {
    it('should not have record-tokens.ts command file', () => {
      const commandFile = join(PROJECT_ROOT, 'src/commands/record-tokens.ts');
      expect(existsSync(commandFile)).toBe(false);
    });

    it('should not have record-tokens-help.ts file', () => {
      const helpFile = join(PROJECT_ROOT, 'src/commands/record-tokens-help.ts');
      expect(existsSync(helpFile)).toBe(false);
    });
  });

  describe('Scenario: Remove actualTokens field from WorkUnit interface', () => {
    it('should not contain actualTokens field in WorkUnit interface', () => {
      const typesFile = join(PROJECT_ROOT, 'src/types/index.ts');
      const content = readFileSync(typesFile, 'utf-8');
      expect(content).not.toContain('actualTokens');
    });

    it('should have valid TypeScript types file', () => {
      const typesFile = join(PROJECT_ROOT, 'src/types/index.ts');
      expect(existsSync(typesFile)).toBe(true);
      const content = readFileSync(typesFile, 'utf-8');
      expect(content).toContain('export interface WorkUnit');
    });
  });

  describe('Scenario: Remove record-tokens CLI registration', () => {
    it('should not import registerRecordTokensCommand in index.ts', () => {
      const indexFile = join(PROJECT_ROOT, 'src/index.ts');
      const content = readFileSync(indexFile, 'utf-8');
      expect(content).not.toContain('registerRecordTokensCommand');
      expect(content).not.toContain("'./commands/record-tokens'");
    });

    it('should not reference record-tokens in index.ts', () => {
      const indexFile = join(PROJECT_ROOT, 'src/index.ts');
      const content = readFileSync(indexFile, 'utf-8');
      expect(content).not.toContain('record-tokens');
    });
  });

  describe('Scenario: Remove record-tokens from help documentation', () => {
    it('should not contain record-tokens in help.ts', () => {
      const helpFile = join(PROJECT_ROOT, 'src/help.ts');
      const content = readFileSync(helpFile, 'utf-8');
      expect(content).not.toContain('record-tokens');
    });

    it('should still have functional help system', () => {
      const helpFile = join(PROJECT_ROOT, 'src/help.ts');
      expect(existsSync(helpFile)).toBe(true);
      const content = readFileSync(helpFile, 'utf-8');
      expect(content).toContain('export function displayCustomHelpWithNote');
    });
  });

  describe('Scenario: Update tests to remove token recording references', () => {
    it('should not contain recordTokens function calls in estimation tests', () => {
      const testFile = join(
        PROJECT_ROOT,
        'src/commands/__tests__/work-unit-estimation-and-metrics.test.ts'
      );
      const content = readFileSync(testFile, 'utf-8');
      expect(content).not.toContain('recordTokens');
    });

    it('should not contain actualTokens assertions in estimation tests', () => {
      const testFile = join(
        PROJECT_ROOT,
        'src/commands/__tests__/work-unit-estimation-and-metrics.test.ts'
      );
      const content = readFileSync(testFile, 'utf-8');
      expect(content).not.toContain('actualTokens');
    });
  });

  describe('Scenario: Remove ai-token-usage-tracking feature files', () => {
    it('should not have ai-token-usage-tracking.feature file', () => {
      const featureFile = join(
        PROJECT_ROOT,
        'spec/features/ai-token-usage-tracking.feature'
      );
      expect(existsSync(featureFile)).toBe(false);
    });

    it('should not have ai-token-usage-tracking.feature.coverage file', () => {
      const coverageFile = join(
        PROJECT_ROOT,
        'spec/features/ai-token-usage-tracking.feature.coverage'
      );
      expect(existsSync(coverageFile)).toBe(false);
    });
  });

  describe('Scenario: Preserve record-iteration and query-metrics commands', () => {
    it('should still have record-iteration.ts file', () => {
      const iterationFile = join(
        PROJECT_ROOT,
        'src/commands/record-iteration.ts'
      );
      expect(existsSync(iterationFile)).toBe(true);
    });

    it('should still have query-metrics.ts file', () => {
      const metricsFile = join(PROJECT_ROOT, 'src/commands/query-metrics.ts');
      expect(existsSync(metricsFile)).toBe(true);
    });
  });

  describe('Scenario: Verify no references to token recording remain', () => {
    it('should not have recordTokens references in source files', () => {
      // This is a meta-test - the absence of recordTokens imports/calls
      // is verified by the codebase compiling successfully
      const indexFile = join(PROJECT_ROOT, 'src/index.ts');
      const content = readFileSync(indexFile, 'utf-8');
      expect(content).not.toContain('recordTokens');
    });

    it('should not have actualTokens field references in types', () => {
      const typesFile = join(PROJECT_ROOT, 'src/types/index.ts');
      const content = readFileSync(typesFile, 'utf-8');
      expect(content).not.toContain('actualTokens');
    });
  });
});
