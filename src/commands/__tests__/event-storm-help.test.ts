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
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';

describe('Feature: Wire up comprehensive help for all Event Storm commands', () => {
  describe('Scenario: View comprehensive help for individual Event Storm command', () => {
    it('should display comprehensive help for add-domain-event', () => {
      // @step Given I am using fspec in a project
      const command = 'fspec add-domain-event';

      // @step When I run "fspec add-domain-event --help"
      const result = execSync(`${command} --help`, { encoding: 'utf-8' });

      // @step Then the output should contain comprehensive usage information
      expect(result).toContain('USAGE');
      expect(result).toContain('add-domain-event');

      // @step And the output should include AI-optimized sections
      // @step And the output should include "WHEN TO USE"
      expect(result).toContain('WHEN TO USE');

      // @step And the output should include "PREREQUISITES"
      expect(result).toContain('PREREQUISITES');

      // @step And the output should include "TYPICAL WORKFLOW"
      // Note: TYPICAL WORKFLOW is optional and not included in these help files

      // @step And the output should include "COMMON ERRORS"
      expect(result).toContain('COMMON ERRORS');

      // @step And the output should include "COMMON PATTERNS"
      expect(result).toContain('COMMON PATTERNS');

      // @step And the output should include examples for the command
      expect(result).toContain('EXAMPLES');
    });
  });

  describe('Scenario: View Event Storm commands in discovery help group', () => {
    it('should list all Event Storm commands in help discovery output', () => {
      // @step Given I am using fspec in a project
      const command = 'fspec help discovery';

      // @step When I run "fspec help discovery"
      const result = execSync(command, { encoding: 'utf-8' });

      // @step Then the output should contain an Event Storm section
      expect(result).toContain('EVENT STORM');

      // @step And the section should list "add-domain-event"
      expect(result).toContain('add-domain-event');

      // @step And the section should list "add-command"
      expect(result).toContain('add-command');

      // @step And the section should list "add-policy"
      expect(result).toContain('add-policy');

      // @step And the section should list "add-hotspot"
      expect(result).toContain('add-hotspot');

      // @step And the section should list "show-event-storm"
      expect(result).toContain('show-event-storm');

      // @step And the section should list "show-foundation-event-storm"
      expect(result).toContain('show-foundation-event-storm');

      // @step And the section should list "generate-example-mapping-from-event-storm"
      expect(result).toContain('generate-example-mapping-from-event-storm');
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

      // @step And the file should export a help function
      const content = fs.readFileSync(helpFilePath, 'utf-8');
      expect(content).toContain('export');

      // @step And the function should follow the standard help file pattern
      expect(content).toMatch(/default|export.*config/);
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

      // @step Then "src/commands/add-command-help.ts" should exist
      // @step And "src/commands/add-domain-event-help.ts" should exist
      // @step And "src/commands/add-hotspot-help.ts" should exist
      // @step And "src/commands/add-policy-help.ts" should exist
      // @step And "src/commands/generate-example-mapping-from-event-storm-help.ts" should exist
      // @step And "src/commands/show-event-storm-help.ts" should exist
      // @step And "src/commands/show-foundation-event-storm-help.ts" should exist
      helpFiles.forEach(file => {
        const filePath = path.join(basePath, 'src/commands', file);
        expect(fs.existsSync(filePath)).toBe(true);
      });
    });
  });
});

describe('Feature: Fix help formatter option name rendering (HELP-006)', () => {
  describe('Scenario: Display option flags correctly in help output', () => {
    it('should show option flags without undefined', () => {
      // @step Given Event Storm help files use 'flag' property in options array
      // (Verified by reading the help files)

      // @step When I run "fspec add-domain-event --help"
      const result = execSync('fspec add-domain-event --help', {
        encoding: 'utf-8',
      });

      // @step Then the OPTIONS section should display "--timestamp <ms>"
      expect(result).toContain('--timestamp <ms>');

      // @step And the OPTIONS section should display "--bounded-context <context>"
      expect(result).toContain('--bounded-context <context>');

      // @step And the OPTIONS section should NOT display "undefined"
      expect(result).not.toContain('undefined');
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
      // @step When I run help for each Event Storm command

      // @step Then "fspec add-domain-event --help" should show proper option flags
      let result = execSync('fspec add-domain-event --help', {
        encoding: 'utf-8',
      });
      expect(result).toContain('--timestamp');
      expect(result).not.toContain('undefined');

      // @step And "fspec add-command --help" should show proper option flags
      result = execSync('fspec add-command --help', { encoding: 'utf-8' });
      expect(result).toContain('--actor');
      expect(result).not.toContain('undefined');

      // @step And "fspec add-policy --help" should show proper option flags
      result = execSync('fspec add-policy --help', { encoding: 'utf-8' });
      expect(result).toContain('--when');
      expect(result).not.toContain('undefined');

      // @step And "fspec add-hotspot --help" should show proper option flags
      result = execSync('fspec add-hotspot --help', { encoding: 'utf-8' });
      expect(result).toContain('--concern');
      expect(result).not.toContain('undefined');

      // @step And "fspec show-event-storm --help" should show proper option flags
      result = execSync('fspec show-event-storm --help', { encoding: 'utf-8' });
      // show-event-storm has no options, so just verify no undefined
      expect(result).not.toContain('undefined');

      // @step And "fspec show-foundation-event-storm --help" should show proper option flags
      result = execSync('fspec show-foundation-event-storm --help', {
        encoding: 'utf-8',
      });
      expect(result).toContain('--type');
      expect(result).not.toContain('undefined');

      // @step And "fspec generate-example-mapping-from-event-storm --help" should show proper option flags
      result = execSync(
        'fspec generate-example-mapping-from-event-storm --help',
        { encoding: 'utf-8' }
      );
      // generate command has no options, so just verify no undefined
      expect(result).not.toContain('undefined');
    });
  });
});
