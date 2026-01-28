/**
 * Feature: spec/features/conversational-test-and-quality-check-tool-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, writeFileSync, existsSync, readFileSync } from 'fs';
import { join } from 'path';
import configureToolsHelpConfig from '../configure-tools-help';
import { formatCommandHelp } from '../../utils/help-formatter';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Conversational Test and Quality Check Tool Detection', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('configure-tools');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Emit system-reminder when no test command configured', () => {
    it('should emit system-reminder when spec/fspec-config.json has no tools.test.command', async () => {
      // Given spec/fspec-config.json does not have tools.test.command configured
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When fspec checks for test command during validating phase
      const { checkTestCommand } = await import('../configure-tools');
      const result = await checkTestCommand(testDir);

      // Then fspec should emit system-reminder
      expect(result.type).toBe('system-reminder');

      // And system-reminder should say: 'No test command configured'
      expect(result.message).toContain('No test command configured');

      // And system-reminder should tell AI to use Read/Glob tools to detect framework
      expect(result.message).toContain('Read/Glob');

      // And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>
      expect(result.message).toContain('fspec configure-tools --test-command');
    });
  });

  describe('Scenario: Store test command when AI configures tools', () => {
    it('should write test command to spec/fspec-config.json when AI runs configure-tools', async () => {
      // Given AI detected test framework using Read/Glob tools
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When AI runs: fspec configure-tools --test-command '<detected-command>'
      const { configureTools } = await import('../configure-tools');
      await configureTools({ testCommand: 'npm test', cwd: testDir });

      // Then fspec should write to spec/fspec-config.json
      expect(existsSync(configPath)).toBe(true);

      // And spec/fspec-config.json should contain tools.test.command = '<detected-command>'
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.test?.command).toBe('npm test');
    });
  });

  describe('Scenario: Emit configured test command when config exists', () => {
    it('should emit configured test command when config already has tools.test.command', async () => {
      // Given spec/fspec-config.json has tools.test.command = '<command>'
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(
        configPath,
        JSON.stringify(
          {
            agent: 'claude',
            tools: { test: { command: 'cargo test' } },
          },
          null,
          2
        )
      );

      // When fspec checks for test command during validating phase
      const { checkTestCommand } = await import('../configure-tools');
      const result = await checkTestCommand(testDir);

      // Then fspec should emit system-reminder: 'Run tests: <command>'
      expect(result.type).toBe('system-reminder');
      expect(result.message).toContain('Run tests: cargo test');

      // And fspec should NOT prompt AI to configure tools
      expect(result.message).not.toContain('fspec configure-tools');
    });
  });

  describe('Scenario: Chain multiple test frameworks when AI provides chained command', () => {
    it('should store chained command when AI provides multiple frameworks', async () => {
      // Given AI detected multiple test frameworks using Glob
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When AI runs: fspec configure-tools --test-command '<framework1> && <framework2>'
      const { configureTools } = await import('../configure-tools');
      await configureTools({
        testCommand: 'vitest && jest',
        cwd: testDir,
      });

      // Then fspec should store chained command in spec/fspec-config.json
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.test?.command).toBe('vitest && jest');

      // And next validation should emit: 'Run tests: <framework1> && <framework2>'
      const { checkTestCommand } = await import('../configure-tools');
      const result = await checkTestCommand(testDir);
      expect(result.message).toContain('Run tests: vitest && jest');
    });
  });

  describe('Scenario: Include date-aware search queries when no tools detected', () => {
    it('should include date-aware search query when no test configuration found', async () => {
      // Given AI used Read/Glob and found no test configuration files
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When fspec emits system-reminder for tool configuration
      const { checkTestCommand } = await import('../configure-tools');
      const result = await checkTestCommand(testDir);

      // Then system-reminder should include search query: 'best <platform> testing tools'
      expect(result.message).toMatch(/best .* testing tools/);

      // And platform placeholder should be filled based on project detection
      // (This will be implemented to detect platform from project files)
      expect(result.message).toContain('testing tools');
    });
  });

  describe('Scenario: Store multiple quality check commands', () => {
    it('should store array of quality commands when AI provides multiple tools', async () => {
      // Given AI detected quality check tools using Glob
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When AI runs: fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>'
      const { configureTools } = await import('../configure-tools');
      await configureTools({
        qualityCommands: ['eslint .', 'tsc --noEmit', 'prettier --check .'],
        cwd: testDir,
      });

      // Then fspec should store array of commands in spec/fspec-config.json
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.qualityCheck?.commands).toEqual([
        'eslint .',
        'tsc --noEmit',
        'prettier --check .',
      ]);

      // And next validation should emit chained command: '<tool1> && <tool2> && <tool3>'
      const { checkQualityCommands } = await import('../configure-tools');
      const result = await checkQualityCommands(testDir);
      expect(result.message).toContain(
        'eslint . && tsc --noEmit && prettier --check .'
      );
    });
  });

  describe('Scenario: Support reconfiguration when tools change', () => {
    it('should re-emit system-reminder when AI runs configure-tools --reconfigure', async () => {
      // Given spec/fspec-config.json has existing tool configuration
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(
        configPath,
        JSON.stringify(
          {
            agent: 'claude',
            tools: { test: { command: 'jest' } },
          },
          null,
          2
        )
      );

      // When AI runs: fspec configure-tools --reconfigure
      const { configureTools } = await import('../configure-tools');
      const result = await configureTools({
        reconfigure: true,
        cwd: testDir,
      });

      // Then fspec should emit system-reminder for re-detection
      expect(result.type).toBe('system-reminder');
      expect(result.message).toContain('Use Read/Glob');

      // And AI should detect new tools and update configuration
      await configureTools({
        testCommand: 'vitest',
        cwd: testDir,
      });
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.test?.command).toBe('vitest');
    });
  });

  describe('Scenario: configure-tools command appears in help output', () => {
    it('should have configure-tools in help config', () => {
      // @step Given configure-tools command is registered in src/index.ts
      // @step When we check the help config
      const helpOutput = formatCommandHelp(configureToolsHelpConfig);

      // @step Then output should list configure-tools
      expect(helpOutput).toContain('configure-tools');

      // @step And help should mention platform-agnostic
      expect(helpOutput).toContain('platform-agnostic');
    });
  });

  describe('Scenario: configure-tools command shows usage documentation', () => {
    it('should display comprehensive help documentation for configure-tools', () => {
      // Given configure-tools command is registered with help documentation
      // When we check the help config
      const helpOutput = formatCommandHelp(configureToolsHelpConfig);

      // Then output should show usage: fspec configure-tools [options]
      expect(helpOutput).toContain('configure-tools');
      expect(helpOutput).toContain('[options]');

      // And output should document --test-command option
      expect(helpOutput).toContain('--test-command');

      // And output should document --quality-commands option
      expect(helpOutput).toContain('--quality-commands');

      // And output should document --reconfigure flag
      expect(helpOutput).toContain('--reconfigure');

      // And output should include examples with different platforms
      expect(helpOutput).toMatch(/npm test|pytest|cargo test/);
    });
  });

  describe('Scenario: configure-tools command stores test command in config file', () => {
    it('should preserve existing agent config when adding test command', async () => {
      // Given spec/fspec-config.json exists with agent configuration
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'cursor' }, null, 2));

      // When user runs 'fspec configure-tools --test-command "npm test"'
      const { configureTools } = await import('../configure-tools');
      await configureTools({ testCommand: 'npm test', cwd: testDir });

      // Then spec/fspec-config.json should be updated with tools.test.command = "npm test"
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.test?.command).toBe('npm test');

      // And existing agent configuration should be preserved
      expect(config.agent).toBe('cursor');
    });
  });

  describe('Scenario: configure-tools command stores quality check commands in config file', () => {
    it('should store quality check commands as array in config file', async () => {
      // Given spec/fspec-config.json exists
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // When user runs 'fspec configure-tools --quality-commands "npm run lint" "npm run typecheck"'
      const { configureTools } = await import('../configure-tools');
      await configureTools({
        qualityCommands: ['npm run lint', 'npm run typecheck'],
        cwd: testDir,
      });

      // Then spec/fspec-config.json should have tools.qualityCheck.commands = ["npm run lint", "npm run typecheck"]
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.qualityCheck?.commands).toEqual([
        'npm run lint',
        'npm run typecheck',
      ]);
    });
  });

  describe('Scenario: configure-tools regenerates agent templates after updating config (CONFIG-003)', () => {
    it('should regenerate spec/CLAUDE.md and .claude/commands/fspec.md with latest templates', async () => {
      // Given spec/fspec-config.json exists with agent = 'claude'
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // And spec/CLAUDE.md exists (old template)
      const oldClaudeMd = '# Old CLAUDE.md template from v0.5.0';
      writeFileSync(join(testDir, 'spec', 'CLAUDE.md'), oldClaudeMd, 'utf-8');

      // And .claude/commands/fspec.md exists (old template)
      mkdirSync(join(testDir, '.claude', 'commands'), { recursive: true });
      const oldFspecMd = '# Old fspec.md template from v0.5.0';
      writeFileSync(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        oldFspecMd,
        'utf-8'
      );

      // When AI runs 'fspec configure-tools --test-command "npm test"'
      const { configureTools } = await import('../configure-tools');
      await configureTools({ testCommand: 'npm test', cwd: testDir });

      // Then config should be updated with tools.test.command = "npm test"
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.tools?.test?.command).toBe('npm test');

      // And spec/CLAUDE.md should be regenerated with latest template
      const updatedClaudeMd = readFileSync(
        join(testDir, 'spec', 'CLAUDE.md'),
        'utf-8'
      );
      expect(updatedClaudeMd).not.toBe(oldClaudeMd);
      expect(updatedClaudeMd.length).toBeGreaterThan(1000); // Latest template is comprehensive

      // And .claude/commands/fspec.md should be regenerated with latest template
      const updatedFspecMd = readFileSync(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(updatedFspecMd).not.toBe(oldFspecMd);
      expect(updatedFspecMd).toContain('fspec --sync-version'); // Latest template includes version sync

      // And templates should reflect current fspec version
      const packageJson = JSON.parse(
        readFileSync(join(process.cwd(), 'package.json'), 'utf-8')
      );
      const currentVersion = packageJson.version;
      expect(updatedFspecMd).toContain(currentVersion);
    });
  });

  describe('Scenario: configure-tools regenerates templates silently without extra output (CONFIG-003)', () => {
    it('should regenerate templates in background without mentioning them in console output', async () => {
      // Given spec/fspec-config.json exists
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }, null, 2));

      // Create old templates
      writeFileSync(
        join(testDir, 'spec', 'CLAUDE.md'),
        '# Old template',
        'utf-8'
      );
      mkdirSync(join(testDir, '.claude', 'commands'), { recursive: true });
      writeFileSync(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        '# Old template',
        'utf-8'
      );

      // Capture console output
      let consoleOutput = '';
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        consoleOutput += args.join(' ') + '\n';
      };

      try {
        // When AI runs 'fspec configure-tools --test-command "npm test"'
        const { configureTools } = await import('../configure-tools');
        await configureTools({ testCommand: 'npm test', cwd: testDir });
      } finally {
        console.log = originalLog;
      }

      // Then console output should NOT contain: 'Regenerating templates'
      // (The configureTools function is designed to be silent during template regeneration)
      expect(consoleOutput).not.toContain('Regenerating templates');
      expect(consoleOutput).not.toContain('regenerating');

      // And console output should NOT contain: '✓ Templates updated'
      expect(consoleOutput).not.toContain('Templates updated');

      // And console output should NOT mention spec/CLAUDE.md
      expect(consoleOutput).not.toContain('spec/CLAUDE.md');

      // And console output should NOT mention .claude/commands/fspec.md
      expect(consoleOutput).not.toContain('.claude/commands/fspec.md');

      // But templates should be regenerated in the background
      const updatedClaudeMd = readFileSync(
        join(testDir, 'spec', 'CLAUDE.md'),
        'utf-8'
      );
      expect(updatedClaudeMd).not.toBe('# Old template');
      expect(updatedClaudeMd.length).toBeGreaterThan(1000);

      const updatedFspecMd = readFileSync(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(updatedFspecMd).not.toBe('# Old template');
      expect(updatedFspecMd).toContain('fspec --sync-version');
    });
  });
});
