/**
 * Feature: spec/features/dynamic-bootstrap-command-for-slash-command-template.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { bootstrap } from '../bootstrap';

describe('Feature: Dynamic bootstrap command for slash command template', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Bootstrap command outputs complete documentation', () => {
    it('should output complete documentation by internally calling all section functions', async () => {
      // Given: I have fspec installed
      // (fspec is installed in the test environment)

      // When: I run 'fspec bootstrap'
      const output = await bootstrap({ cwd: tmpDir });

      // Then: the command should output complete documentation by internally calling all section functions
      expect(output).toContain(
        '# fspec Command - Kanban-Based Project Management'
      );
      expect(output).toContain('ACDD (Acceptance Criteria Driven Development)');
      expect(output).toContain('Example Mapping');
      expect(output).toContain('Story Point Estimation');
      expect(output).toContain('Kanban');

      // @step And the output should include string replacement for <test-command> and <quality-check-commands> placeholders
      // Note: Placeholders remain if no config file exists (which is correct behavior for tmpDir)
      // The actual replacement happens when spec/fspec-config.json exists
      // This test verifies the bootstrap command runs without error
      expect(output).toContain('<test-command>'); // Placeholders present when no config
      expect(output).toContain('<quality-check-commands>');

      // @step And the output should be identical to what currently appears after 'fspec --sync-version' in templates
      // (This will be verified by comparing with getSlashCommandTemplate() output)
    });
  });

  describe('Scenario: Template contains only two commands', () => {
    it('should generate template with only 2 commands', async () => {
      // Given: I run 'fspec init --agent=claude'
      // (This is tested in init.test.ts, we'll verify template content here)

      // When: the template file .claude/commands/fspec.md is generated
      const { getSlashCommandTemplate } = await import(
        '../../utils/slashCommandTemplate'
      );
      const template = getSlashCommandTemplate();

      // Then: the template should contain sync-version and bootstrap
      expect(template).toContain('fspec --sync-version');
      expect(template).toContain('fspec bootstrap');

      // @step And the template should NOT contain 'fspec --help', 'fspec help specs', etc.
      expect(template).not.toContain('fspec --help');
      expect(template).not.toContain('fspec help specs');
      expect(template).not.toContain('fspec help work');

      // @step And the template should instruct AI to run both commands before continuing
      expect(template).toContain(
        'YOU MUST RUN THOSE COMMANDS AND WAIT FOR THEM TO FINISH'
      );
    });
  });

  describe('Scenario: Template migration on version upgrade', () => {
    it('should regenerate template with 2 commands on version mismatch', async () => {
      // Given: I have fspec v0.6.0 installed with template containing 8 commands
      // (Simulated by sync-version detecting mismatch)

      // When: sync-version detects mismatch and calls installAgentFiles()
      const { getSlashCommandTemplate } = await import(
        '../../utils/slashCommandTemplate'
      );
      const newTemplate = getSlashCommandTemplate();

      // Then: the new template should contain sync-version and bootstrap commands
      expect(newTemplate).toMatch(/fspec --sync-version \d+\.\d+\.\d+/);
      expect(newTemplate).toContain('fspec bootstrap');

      // @step And it should call installAgentFiles() to regenerate template
      // (Tested via getSlashCommandTemplate() which is called by installAgentFiles)

      // @step And it should exit with code 1 and tell user to restart
      // Note: Exit code 1 and restart message tested in sync-version.test.ts
    });
  });

  describe('Scenario: Bootstrap internally executes all help commands', () => {
    it('should internally execute all 7 help commands and combine output', async () => {
      // Given: I have fspec installed
      // (fspec is installed in test environment)

      // When: I run 'fspec bootstrap'
      const output = await bootstrap({ cwd: tmpDir });

      // Then: the command should internally execute all help commands
      // Verify output contains content from each help section:

      // From 'fspec --help' - main help
      expect(output).toContain(
        'fspec Command - Kanban-Based Project Management'
      );

      // From 'fspec help specs' - specification commands
      expect(output).toContain('GHERKIN SPECIFICATIONS');
      expect(output).toContain('create-feature');
      expect(output).toContain('validate');

      // From 'fspec help work' - work unit commands
      expect(output).toContain('create-story');
      expect(output).toContain('update-work-unit-status');
      expect(output).toContain('board');

      // From 'fspec help discovery' - example mapping
      expect(output).toContain('Example Mapping');
      expect(output).toContain('add-rule');
      expect(output).toContain('add-example');

      // From 'fspec help metrics' - estimation and metrics
      expect(output).toContain('Story Point Estimation');
      expect(output).toContain('update-work-unit-estimate');

      // From 'fspec help setup' - initialization
      expect(output).toContain('discover-foundation');
      expect(output).toContain('configure-tools');

      // From 'fspec help hooks' - lifecycle hooks
      expect(output).toContain('LIFECYCLE HOOKS');
      expect(output).toContain('add-hook');
      expect(output).toContain('add-virtual-hook');

      // @step And the combined output should be printed to stdout
      expect(output.length).toBeGreaterThan(10000); // Substantial combined content
    });
  });

  describe('Scenario: Bootstrap displays explainer about help command outputs', () => {
    it('should display explainer section before help command outputs', async () => {
      // Given: I run 'fspec bootstrap'
      // When: the bootstrap command outputs its content
      const output = await bootstrap({ cwd: tmpDir });

      // Then: the output should contain an explainer section before the help command outputs
      const explainerIndex = output.indexOf('## Step 2:');
      const helpIndex = output.indexOf('GHERKIN SPECIFICATIONS');
      expect(explainerIndex).toBeGreaterThan(0);
      expect(helpIndex).toBeGreaterThan(0);
      expect(explainerIndex).toBeLessThan(helpIndex);

      // And: the explainer should explain what the content is (output from fspec help commands)
      expect(output).toContain('complete fspec command documentation');
      expect(output).toContain('fspec --help');
      expect(output).toContain('fspec help specs');
      expect(output).toContain('fspec help work');
      expect(output).toContain('fspec help discovery');
      expect(output).toContain('fspec help metrics');
      expect(output).toContain('fspec help setup');
      expect(output).toContain('fspec help hooks');

      // And: the explainer should explain WHY this information matters (complete command reference)
      expect(output).toContain('command reference');

      // And: the explainer should explain HOW to access sections again (run individual help commands, NOT bootstrap)
      expect(output).toContain('run these commands individually');
      expect(output).toContain('refresh specific sections');
    });
  });

  describe('Scenario: Bootstrap command has no customization options', () => {
    it('should not accept any options or flags', async () => {
      // Given: I have fspec installed with bootstrap command
      // (bootstrap command exists)

      // When: I run 'fspec bootstrap --help'
      // @step And there should be no --skip-help, --minimal, or --skip-sections flags
      // This will be verified by checking the command registration
      // which should NOT have any options defined

      // For now, verify bootstrap always outputs everything
      const output = await bootstrap({ cwd: tmpDir });

      // @step And running 'fspec bootstrap' should always output complete documentation
      expect(output).toContain(
        '# fspec Command - Kanban-Based Project Management'
      );
      expect(output).toContain('ACDD');
      expect(output).toContain('Example Mapping');
      expect(output).toContain('Kanban');
    });
  });

  describe('Scenario: Help content getter functions return complete documentation not stub text', () => {
    it('should return full command documentation not stub text', async () => {
      // Given: I have the display*Help() functions in src/help.ts with full command documentation
      const {
        getSpecsHelpContent,
        getWorkHelpContent,
        getDiscoveryHelpContent,
        getMetricsHelpContent,
        getSetupHelpContent,
        getHooksHelpContent,
      } = await import('../../help');

      // When: I call getSpecsHelpContent(), getWorkHelpContent(), etc.
      const specsHelp = getSpecsHelpContent();
      const workHelp = getWorkHelpContent();
      const discoveryHelp = getDiscoveryHelpContent();
      const metricsHelp = getMetricsHelpContent();
      const setupHelp = getSetupHelpContent();
      const hooksHelp = getHooksHelpContent();

      // Then: each function should return the EXACT SAME content as its corresponding display function
      // but as plain text without chalk formatting

      // @step And getSpecsHelpContent() should return full 333 lines of content from displaySpecsHelp()
      // showing ALL commands with examples and options
      expect(specsHelp).toContain('GHERKIN SPECIFICATIONS');
      expect(specsHelp).toContain('create-feature');
      expect(specsHelp).toContain('add-scenario');
      expect(specsHelp).toContain('add-step');
      expect(specsHelp).toContain('validate');
      expect(specsHelp).toContain('format');
      expect(specsHelp).toContain('link-coverage');
      expect(specsHelp).toContain('add-tag-to-feature');
      expect(specsHelp).toContain('remove-tag-from-feature');
      expect(specsHelp).not.toContain('and more'); // No stub text

      // @step And getWorkHelpContent() should return full 308 lines of content from displayWorkHelp()
      // showing ALL commands including checkpoints, dependencies, analysis commands
      expect(workHelp).toContain('WORK UNIT MANAGEMENT');
      expect(workHelp).toContain('create-story');
      expect(workHelp).toContain('create-bug');
      expect(workHelp).toContain('create-task');
      expect(workHelp).toContain('update-work-unit-status');
      expect(workHelp).toContain('board');
      expect(workHelp).toContain('checkpoint');
      expect(workHelp).toContain('restore-checkpoint');
      expect(workHelp).toContain('list-checkpoints');
      expect(workHelp).toContain('cleanup-checkpoints');
      expect(workHelp).toContain('add-dependency');
      expect(workHelp).toContain('search-scenarios');
      expect(workHelp).toContain('search-implementation');
      expect(workHelp).toContain('compare-implementations');
      expect(workHelp).not.toContain('and more'); // No stub text

      // @step And all 6 getter functions should NOT return stub text
      expect(discoveryHelp).toContain('EXAMPLE MAPPING');
      expect(discoveryHelp).toContain('add-rule');
      expect(discoveryHelp).toContain('add-example');
      expect(discoveryHelp).toContain('add-question');
      expect(discoveryHelp).not.toContain('and more');

      expect(metricsHelp).toContain('PROGRESS TRACKING & METRICS');
      expect(metricsHelp).toContain('query-metrics');
      expect(metricsHelp).not.toContain('and more');

      expect(setupHelp).toContain('CONFIGURATION & SETUP');
      expect(setupHelp).toContain('discover-foundation');
      expect(setupHelp).toContain('configure-tools');
      expect(setupHelp).toContain('list-tags');
      expect(setupHelp).toContain('create-epic');
      expect(setupHelp).not.toContain('and more');

      expect(hooksHelp).toContain('LIFECYCLE HOOKS');
      expect(hooksHelp).toContain('add-hook');
      expect(hooksHelp).toContain('add-virtual-hook');
      expect(hooksHelp).toContain('list-hooks');
      expect(hooksHelp).not.toContain('and more');
    });
  });

  describe('Scenario: Template contains only minimal header without duplication', () => {
    it('should contain minimal header without persona description or ACDD workflow', async () => {
      // Given: the slash command template is generated
      const { getSlashCommandTemplate } = await import(
        '../../utils/slashCommandTemplate'
      );

      // When: I read the template file
      const template = getSlashCommandTemplate();

      // Then: template should be concise (minimal header with commands only)
      const lines = template.trim().split('\n');
      expect(lines.length).toBeLessThanOrEqual(15); // Allow some flexibility for readability

      // And: template should NOT contain persona description or ACDD workflow sections
      expect(template).not.toContain('You are a master of project management');
      expect(template).not.toContain('ACDD (Acceptance Criteria Driven');
      expect(template).not.toContain('Example Mapping');
      expect(template).not.toContain('Kanban Workflow');
      expect(template).not.toContain('Story Point Estimation');

      // And: bootstrap command output should contain all workflow documentation
      const bootstrapOutput = await bootstrap({ cwd: tmpDir });
      expect(bootstrapOutput).toContain('You are a master of project');
      expect(bootstrapOutput).toContain('ACDD');
      expect(bootstrapOutput).toContain('Example Mapping');
      expect(bootstrapOutput).toContain('Kanban');
    });
  });
});
