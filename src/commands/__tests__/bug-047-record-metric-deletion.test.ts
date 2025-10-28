/**
 * Feature: spec/features/type-mismatches-in-record-metric-and-record-tokens-commands.feature
 *
 * This test file validates that record-metric command has been completely removed
 * and record-tokens function signature has been fixed.
 *
 * These tests should FAIL until the implementation is complete (TDD red phase).
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';

describe('Feature: Type mismatches in record-metric and record-tokens commands', () => {
  describe('Scenario: Delete duplicate record-metric command', () => {
    it('should not have record-metric.ts file', () => {
      // @step Given record-metric.ts exists with identical implementation to record-tokens
      // @step And record-metric has incorrect command signature (<metric> <value>)
      // @step When I delete src/commands/record-metric.ts
      // @step Then the duplicate command should be removed
      const filePath = join(process.cwd(), 'src/commands/record-metric.ts');
      expect(existsSync(filePath)).toBe(false);
    });

    it('should not have record-metric-help.ts file', () => {
      // @step And I delete src/commands/record-metric-help.ts
      // @step Then the duplicate command should be removed
      const filePath = join(
        process.cwd(),
        'src/commands/record-metric-help.ts'
      );
      expect(existsSync(filePath)).toBe(false);
    });
  });

  describe('Scenario: Remove record-metric from command registration', () => {
    it('should not import recordMetric in src/index.ts', async () => {
      // @step Given record-metric is registered in src/index.ts
      // @step When I remove the recordMetric import
      // @step Then record-metric command should no longer be available
      const filePath = join(process.cwd(), 'src/index.ts');
      const content = await readFile(filePath, 'utf-8');

      // Should not have recordMetric import
      expect(content).not.toMatch(/import.*recordMetric.*from/);
      expect(content).not.toMatch(/from ['"]\.\/commands\/record-metric['"]/);
    });

    it('should not call registerRecordMetricCommand in src/index.ts', async () => {
      // @step And I remove the registerRecordMetricCommand call
      // @step Then record-metric command should no longer be available
      const filePath = join(process.cwd(), 'src/index.ts');
      const content = await readFile(filePath, 'utf-8');

      // Should not have registerRecordMetricCommand call
      expect(content).not.toMatch(/registerRecordMetricCommand/);
    });

    it('should not reference record-metric in src/help.ts', async () => {
      // @step Then record-metric command should no longer be available
      const filePath = join(process.cwd(), 'src/help.ts');
      const content = await readFile(filePath, 'utf-8');

      // Should not reference record-metric
      expect(content).not.toMatch(/record-metric/i);
    });
  });

  describe('Scenario: Update documentation to remove record-metric references', () => {
    it('should not reference record-metric in work-unit-estimation-and-metrics.feature', async () => {
      // @step Given record-metric is referenced in feature files
      // @step And record-metric is referenced in test files
      // @step When I update work-unit-estimation-and-metrics.feature to use record-tokens
      // @step Then all documentation should only reference record-tokens
      const filePath = join(
        process.cwd(),
        'spec/features/work-unit-estimation-and-metrics.feature'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not mention record-metric
      expect(content).not.toMatch(/record-metric/i);
    });

    it('should not reference record-metric in cli-command-registration.feature', async () => {
      // @step And I update cli-command-registration.feature to remove record-metric
      // @step Then all documentation should only reference record-tokens
      const filePath = join(
        process.cwd(),
        'spec/features/cli-command-registration.feature'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not mention record-metric
      expect(content).not.toMatch(/record-metric/i);
    });

    it('should not reference record-metric in query-metrics-help.ts', async () => {
      // @step Then all documentation should only reference record-tokens
      const filePath = join(
        process.cwd(),
        'src/commands/query-metrics-help.ts'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not mention record-metric
      expect(content).not.toMatch(/record-metric/i);
    });

    it('should not reference record-metric in test files', async () => {
      // @step And I update test files to remove record-metric tests
      // @step Then all documentation should only reference record-tokens
      const testFiles = [
        'src/commands/__tests__/cli-command-registration.test.ts',
        'src/commands/__tests__/bootstrap.test.ts',
        'src/commands/__tests__/work-unit-estimation-and-metrics.test.ts',
      ];

      for (const file of testFiles) {
        const filePath = join(process.cwd(), file);
        const content = await readFile(filePath, 'utf-8');

        // Should not mention record-metric
        expect(content).not.toMatch(/record-metric/i);
      }
    });

    it('should not reference record-metric in LOCK-002 checklist', async () => {
      // @step Then all documentation should only reference record-tokens
      const filePath = join(
        process.cwd(),
        'spec/attachments/LOCK-002/remaining-refactoring-checklist.md'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not mention record-metric
      expect(content).not.toMatch(/record-metric/i);
    });
  });

  describe('Scenario: Remove record-metric from coverage files', () => {
    it('should not reference record-metric in coverage file', async () => {
      // @step Given record-metric is tracked in coverage files
      // @step When I update work-unit-estimation-and-metrics.feature.coverage
      // @step Then coverage should only track record-tokens
      const filePath = join(
        process.cwd(),
        'spec/features/work-unit-estimation-and-metrics.feature.coverage'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not mention record-metric
      expect(content).not.toMatch(/record-metric/i);
    });
  });

  describe('Scenario: Add operation parameter to record-tokens function', () => {
    it('should have operation parameter in recordTokens function signature', async () => {
      // @step Given record-tokens command passes operation option
      // @step But recordTokens function doesn't accept operation parameter
      // @step When I add operation?: string to function signature
      // @step Then command and function signatures should match
      const filePath = join(process.cwd(), 'src/commands/record-tokens.ts');
      const content = await readFile(filePath, 'utf-8');

      // Should have operation parameter in function signature
      // Looking for: operation?: string
      expect(content).toMatch(/operation\?:\s*string/);
    });
  });
});
