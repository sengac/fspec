/**
 * Feature: spec/features/wire-up-comprehensive-help-for-all-event-storm-commands.feature
 *
 * Tests for Event Storm command help integration.
 * Ensures all 7 Event Storm commands have comprehensive help with AI-optimized sections.
 */

/**
 * Feature: spec/features/fix-help-formatter-option-name-rendering-and-commonerrors-property-mismatch.feature
 *
 * BUG HELP-006: Fix help formatter option name rendering issues
 */

import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';

// Import help configs directly - no need to spawn processes
import addDomainEventHelp from '../add-domain-event-help';
import addCommandHelp from '../add-command-help';
import addPolicyHelp from '../add-policy-help';
import addHotspotHelp from '../add-hotspot-help';
import showEventStormHelp from '../show-event-storm-help';
import showFoundationEventStormHelp from '../show-foundation-event-storm-help';
import generateExampleMappingFromEventStormHelp from '../generate-example-mapping-from-event-storm-help';
import { formatCommandHelp } from '../../utils/help-formatter';

describe('Feature: Wire up comprehensive help for all Event Storm commands', () => {
  describe('Scenario: View comprehensive help for individual Event Storm command', () => {
    it('should have comprehensive help config for add-domain-event', () => {
      // @step Given I am using fspec in a project
      const config = addDomainEventHelp;

      // @step Then the config should contain comprehensive usage information
      expect(config.usage).toContain('add-domain-event');

      // @step And the config should include AI-optimized sections
      // @step And the config should include "whenToUse"
      expect(config.whenToUse).toBeDefined();
      expect(config.whenToUse!.length).toBeGreaterThan(0);

      // @step And the config should include "prerequisites"
      expect(config.prerequisites).toBeDefined();
      expect(config.prerequisites!.length).toBeGreaterThan(0);

      // @step And the config should include "commonErrors"
      expect(config.commonErrors).toBeDefined();
      expect(config.commonErrors!.length).toBeGreaterThan(0);

      // @step And the config should include "commonPatterns"
      expect(config.commonPatterns).toBeDefined();

      // @step And the config should include examples for the command
      expect(config.examples).toBeDefined();
      expect(config.examples!.length).toBeGreaterThan(0);

      // Verify formatted output contains expected sections
      const formatted = formatCommandHelp(config);
      expect(formatted).toContain('USAGE');
      expect(formatted).toContain('WHEN TO USE');
      expect(formatted).toContain('PREREQUISITES');
      expect(formatted).toContain('COMMON ERRORS');
      expect(formatted).toContain('EXAMPLES');
    });
  });

  describe('Scenario: Verify help file exists for generate-example-mapping-from-event-storm', () => {
    it('should have help file with proper structure', () => {
      // @step Given the fspec codebase
      const basePath = process.cwd();

      // @step When I check for the file "src/commands/generate-example-mapping-from-event-storm-help.ts"
      const helpFilePath = path.join(
        basePath,
        'src/commands/generate-example-mapping-from-event-storm-help.ts'
      );

      // @step Then the file should exist
      expect(fs.existsSync(helpFilePath)).toBe(true);

      // @step And the imported config should be valid
      expect(generateExampleMappingFromEventStormHelp.name).toBe(
        'generate-example-mapping-from-event-storm'
      );
    });
  });

  describe('Scenario: Verify all 7 Event Storm commands have help files', () => {
    it('should have help files for all Event Storm commands', () => {
      // @step Given the fspec codebase
      const basePath = process.cwd();

      // @step When I check for help files for all Event Storm commands
      const helpFiles = [
        'add-command-help.ts',
        'add-domain-event-help.ts',
        'add-hotspot-help.ts',
        'add-policy-help.ts',
        'generate-example-mapping-from-event-storm-help.ts',
        'show-event-storm-help.ts',
        'show-foundation-event-storm-help.ts',
      ];

      // @step Then all help files should exist
      helpFiles.forEach(file => {
        const filePath = path.join(basePath, 'src/commands', file);
        expect(fs.existsSync(filePath)).toBe(true);
      });

      // Verify all configs are importable and have required fields
      const configs = [
        addCommandHelp,
        addDomainEventHelp,
        addHotspotHelp,
        addPolicyHelp,
        generateExampleMappingFromEventStormHelp,
        showEventStormHelp,
        showFoundationEventStormHelp,
      ];

      configs.forEach(config => {
        expect(config.name).toBeDefined();
        expect(config.description).toBeDefined();
        expect(config.usage).toBeDefined();
      });
    });
  });
});

describe('Feature: Fix help formatter option name rendering (HELP-006)', () => {
  describe('Scenario: Display option flags correctly in help output', () => {
    it('should show option flags without undefined', () => {
      // @step Given Event Storm help files use 'flag' property in options array
      const config = addDomainEventHelp;

      // @step Then the formatted output should display "--timestamp <ms>"
      const formatted = formatCommandHelp(config);
      expect(formatted).toContain('--timestamp <ms>');

      // @step And the formatted output should display "--bounded-context <context>"
      expect(formatted).toContain('--bounded-context <context>');

      // @step And the formatted output should NOT display "undefined"
      expect(formatted).not.toContain('undefined');
    });
  });

  describe('Scenario: Use fix property consistently in commonErrors', () => {
    it('should use fix property not solution in all Event Storm help files', () => {
      // @step Given all Event Storm help files exist
      const helpFiles = [
        'add-command-help.ts',
        'add-domain-event-help.ts',
        'add-hotspot-help.ts',
        'add-policy-help.ts',
        'generate-example-mapping-from-event-storm-help.ts',
        'show-event-storm-help.ts',
        'show-foundation-event-storm-help.ts',
      ];

      const basePath = process.cwd();

      // @step When I check the commonErrors array in each help file
      // @step Then all commonErrors should use 'fix' property
      // @step And no commonErrors should use 'solution' property
      helpFiles.forEach(file => {
        const filePath = path.join(basePath, 'src/commands', file);
        const content = fs.readFileSync(filePath, 'utf-8');

        // Check for 'fix:' property in commonErrors
        if (content.includes('commonErrors')) {
          expect(content).toMatch(/fix:/);
          expect(content).not.toMatch(/solution:/);
        }
      });
    });
  });

  describe('Scenario: Verify all 7 Event Storm help files are fixed', () => {
    it('should show proper option flags for all commands', () => {
      // @step Given the Event Storm help files have been updated
      // @step When I format help for each Event Storm command

      // @step Then add-domain-event should show proper option flags
      let formatted = formatCommandHelp(addDomainEventHelp);
      expect(formatted).toContain('--timestamp');
      expect(formatted).not.toContain('undefined');

      // @step And add-command should show proper option flags
      formatted = formatCommandHelp(addCommandHelp);
      expect(formatted).toContain('--actor');
      expect(formatted).not.toContain('undefined');

      // @step And add-policy should show proper option flags
      formatted = formatCommandHelp(addPolicyHelp);
      expect(formatted).toContain('--when');
      expect(formatted).not.toContain('undefined');

      // @step And add-hotspot should show proper option flags
      formatted = formatCommandHelp(addHotspotHelp);
      expect(formatted).toContain('--concern');
      expect(formatted).not.toContain('undefined');

      // @step And show-event-storm should not contain undefined
      formatted = formatCommandHelp(showEventStormHelp);
      expect(formatted).not.toContain('undefined');

      // @step And show-foundation-event-storm should show proper option flags
      formatted = formatCommandHelp(showFoundationEventStormHelp);
      expect(formatted).toContain('--type');
      expect(formatted).not.toContain('undefined');

      // @step And generate-example-mapping-from-event-storm should not contain undefined
      formatted = formatCommandHelp(generateExampleMappingFromEventStormHelp);
      expect(formatted).not.toContain('undefined');
    });
  });
});
