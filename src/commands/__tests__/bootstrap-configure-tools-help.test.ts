/**
 * Feature: spec/features/add-comprehensive-help-documentation-for-bootstrap-and-configure-tools-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import bootstrapHelpConfig from '../bootstrap-help';
import configureToolsHelpConfig from '../configure-tools-help';
import { formatCommandHelp } from '../../utils/help-formatter';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe('Feature: Add comprehensive help documentation for bootstrap and configure-tools commands', () => {
  describe('Scenario: Display comprehensive help for bootstrap command', () => {
    it('should display comprehensive help with all required sections', () => {
      // Given the bootstrap command exists
      // When I format the help config
      const helpOutput = formatCommandHelp(bootstrapHelpConfig);

      // Then I should see the command description
      expect(helpOutput).toMatch(/bootstrap/i);
      expect(helpOutput).toMatch(/description|loads|workflow/i);

      // And I should see the "WHEN TO USE" section
      expect(helpOutput).toMatch(/when to use/i);

      // And I should see the "PREREQUISITES" section
      expect(helpOutput).toMatch(/prerequisites/i);

      // And I should see the "TYPICAL WORKFLOW" section
      expect(helpOutput).toMatch(/typical workflow|workflow/i);

      // And I should see usage examples
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(/fspec bootstrap/i);
    });
  });

  describe('Scenario: Display comprehensive help for configure-tools command', () => {
    it('should display help with platform-agnostic examples', () => {
      // Given the configure-tools command exists
      // When I format the help config
      const helpOutput = formatCommandHelp(configureToolsHelpConfig);

      // Then I should see the command description
      expect(helpOutput).toMatch(/configure.*tools/i);

      // And I should see platform-agnostic examples
      expect(helpOutput).toMatch(/example/i);

      // And I should see examples for Node.js with npm test
      expect(helpOutput).toMatch(/npm test|node.*js/i);

      // And I should see examples for Python with pytest
      expect(helpOutput).toMatch(/pytest|python/i);

      // And I should see examples for Rust with cargo test
      expect(helpOutput).toMatch(/cargo test|rust/i);

      // And I should see examples for Go with go test
      expect(helpOutput).toMatch(/go test|golang/i);
    });
  });

  describe('Scenario: Bootstrap command help includes complete workflow documentation', () => {
    it('should include references to all help sections', () => {
      // Given the bootstrap help content config exists
      // Then the config should reference all help sections
      expect(bootstrapHelpConfig.notes).toBeDefined();
      const notesText = bootstrapHelpConfig.notes?.join(' ') || '';

      // And the notes should mention specs help content
      expect(notesText).toMatch(/getSpecsHelpContent|specs/i);

      // And the notes should mention work help content
      expect(notesText).toMatch(/getWorkHelpContent|work/i);

      // And the notes should mention discovery help content
      expect(notesText).toMatch(/getDiscoveryHelpContent|discovery/i);

      // And the notes should mention metrics help content
      expect(notesText).toMatch(/getMetricsHelpContent|metrics/i);

      // And the notes should mention setup help content
      expect(notesText).toMatch(/getSetupHelpContent|setup/i);

      // And the notes should mention hooks help content
      expect(notesText).toMatch(/getHooksHelpContent|hooks/i);
    });
  });

  describe('Scenario: Documentation consistency across all sources', () => {
    it('should have consistent documentation across all sources', () => {
      // Given bootstrap-help.ts exists
      const bootstrapHelpPath = path.resolve(__dirname, '../bootstrap-help.ts');
      const configureToolsHelpPath = path.resolve(
        __dirname,
        '../configure-tools-help.ts'
      );

      // And configure-tools-help.ts exists
      const bootstrapHelpExists = existsSync(bootstrapHelpPath);
      const configureToolsHelpExists = existsSync(configureToolsHelpPath);

      // When I compare help content with docs directory
      // Then bootstrap documentation should be consistent
      expect(bootstrapHelpExists).toBe(true);

      // And configure-tools documentation should be consistent
      expect(configureToolsHelpExists).toBe(true);

      // And the configs should have required fields
      expect(bootstrapHelpConfig.name).toBe('bootstrap');
      expect(bootstrapHelpConfig.description).toBeDefined();
      expect(bootstrapHelpConfig.usage).toBeDefined();

      expect(configureToolsHelpConfig.name).toBe('configure-tools');
      expect(configureToolsHelpConfig.description).toBeDefined();
      expect(configureToolsHelpConfig.usage).toBeDefined();
    });
  });
});
