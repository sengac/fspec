/**
 * Feature: spec/features/automatic-version-check-and-update-for-slash-command-files.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { execSync } from 'child_process';

describe('Feature: Automatic version check and update for slash command files', () => {
  let testDir: string;
  const fspecBin = join(process.cwd(), 'dist/index.js');

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-sync-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Version mismatch detected on upgrade (Claude Code)', () => {
    it('should detect version mismatch, update files, show Claude-specific restart message, and exit with code 1', async () => {
      // Given I have fspec v0.5.0 installed with .claude/commands/fspec.md containing "fspec --sync-version 0.5.0"
      await mkdir(join(testDir, '.claude', 'commands'), { recursive: true });
      const oldFspecMd = `# fspec Command - Kanban-Based Project Management

fspec --sync-version 0.5.0

IMMEDIATELY - run these commands and store them into your context:
...old content...`;
      await writeFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        oldFspecMd,
        'utf-8'
      );

      // And spec/fspec-config.json contains agent "claude"
      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify({ agent: 'claude' }),
        'utf-8'
      );

      // And spec/CLAUDE.md exists with old content
      const oldClaudeMd = '# Old CLAUDE.md content from v0.5.0';
      await writeFile(join(testDir, 'spec', 'CLAUDE.md'), oldClaudeMd, 'utf-8');

      // @step When I upgrade to fspec v0.6.0 with "npm install -g @sengac/fspec@0.6.0"
      // (Simulated by having current version in package.json different from 0.5.0)

      // @step And I run /fspec in Claude Code
      // @step And the AI agent executes "fspec --sync-version 0.5.0" as the first command
      let exitCode = 0;
      let output = '';
      try {
        output = execSync(`node ${fspecBin} --sync-version 0.5.0`, {
          cwd: testDir,
          encoding: 'utf-8',
          stdio: 'pipe',
        });
      } catch (error: any) {
        exitCode = error.status || 1;
        output = error.stdout || error.stderr || '';
      }

      // Then the command should detect version mismatch (0.6.0 != current package.json version)
      // NOTE: This test will FAIL in RED phase because --sync-version command doesn't exist yet
      // @step And it should exit with code 1 (stopping workflow)
      expect(exitCode).toBe(1);

      // And it should update .claude/commands/fspec.md with new content including current version
      const updatedFspecMd = await readFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(updatedFspecMd).toContain('fspec --sync-version');
      expect(updatedFspecMd).not.toContain('0.5.0'); // Should no longer have old version
      expect(updatedFspecMd).toContain('0.6.0'); // Should have current version now
      expect(updatedFspecMd).not.toContain('...old content...');

      // And it should update spec/CLAUDE.md with latest documentation
      const updatedClaudeMd = await readFile(
        join(testDir, 'spec', 'CLAUDE.md'),
        'utf-8'
      );
      expect(updatedClaudeMd).not.toBe(oldClaudeMd);
      expect(updatedClaudeMd.length).toBeGreaterThan(1000); // Latest doc should be comprehensive

      // @step And it should print "⚠️  fspec files updated from v0.5.0 to v0.6.0"
      expect(output).toContain('⚠️');
      expect(output).toContain('fspec files updated from v0.5.0');

      // And it should print Claude-specific restart message wrapped in <system-reminder> tags
      expect(output).toContain('<system-reminder>');
      // @step And the restart message should say "Exit this conversation and start a new one. Run /fspec again."
      expect(output).toContain('Exit this conversation and start a new one');
      expect(output).toContain('Run /fspec again');
      expect(output).toContain('</system-reminder>');

      // @step And the AI agent should stop loading further help commands
      // (Verified by exit code 1 which stops execution)
    });
  });

  describe('Scenario: Version match - no update needed', () => {
    it('should detect version match, not update files, print nothing, and exit with code 0', async () => {
      // Given I have fspec with current version in fspec.md
      await mkdir(join(testDir, '.claude', 'commands'), { recursive: true });

      // Read actual package.json version
      const packageJson = JSON.parse(
        await readFile(join(process.cwd(), 'package.json'), 'utf-8')
      );
      const currentVersion = packageJson.version;

      const currentFspecMd = `# fspec Command - Kanban-Based Project Management

fspec --sync-version ${currentVersion}

IMMEDIATELY - run these commands and store them into your context:
...current content...`;
      await writeFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        currentFspecMd,
        'utf-8'
      );

      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify({ agent: 'claude' }),
        'utf-8'
      );

      const originalContent = currentFspecMd;

      // @step When I run /fspec in Claude Code
      // @step And the AI agent executes "fspec --sync-version 0.7.0" as the first command
      let exitCode = 1; // Default to failure
      let output = '';
      try {
        output = execSync(`node ${fspecBin} --sync-version ${currentVersion}`, {
          cwd: testDir,
          encoding: 'utf-8',
          stdio: 'pipe',
        });
        exitCode = 0;
      } catch (error: any) {
        exitCode = error.status || 1;
        output = error.stdout || error.stderr || '';
      }

      // Then the command should detect version match
      // NOTE: This test will FAIL in RED phase
      // @step And it should exit with code 0 (continuing workflow)
      expect(exitCode).toBe(0);

      // And it should not update any files
      const unchangedContent = await readFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(unchangedContent).toBe(originalContent);

      // And it should not print any output (silent)
      expect(output.trim()).toBe('');

      // @step And the AI agent should proceed to load help commands normally
      // (Verified by exit code 0 which allows workflow to continue)
    });
  });

  describe('Scenario: Version mismatch detected for Cursor agent (no system-reminders)', () => {
    it('should update Cursor files and show plain text restart message (no system-reminder tags)', async () => {
      // Given I have fspec v0.5.0 installed with .cursor/commands/fspec.md
      await mkdir(join(testDir, '.cursor', 'commands'), { recursive: true });
      const oldFspecMd = `# fspec Command

fspec --sync-version 0.5.0

Old cursor content...`;
      await writeFile(
        join(testDir, '.cursor', 'commands', 'fspec.md'),
        oldFspecMd,
        'utf-8'
      );

      // And spec/fspec-config.json contains agent "cursor"
      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify({ agent: 'cursor' }),
        'utf-8'
      );

      // And Cursor agent does not support system-reminders
      // (This is implicit - cursor has supportsSystemReminders: false in agentRegistry)

      await writeFile(
        join(testDir, 'spec', 'CURSOR.md'),
        '# Old CURSOR.md',
        'utf-8'
      );

      // @step When I upgrade to fspec v0.6.0
      // @step And I run /fspec in Cursor
      // @step And the AI agent executes "fspec --sync-version 0.5.0" as the first command
      let exitCode = 0;
      let output = '';
      try {
        output = execSync(`node ${fspecBin} --sync-version 0.5.0`, {
          cwd: testDir,
          encoding: 'utf-8',
          stdio: 'pipe',
        });
      } catch (error: any) {
        exitCode = error.status || 1;
        output = error.stdout || error.stderr || '';
      }

      // Then the command should detect version mismatch
      // NOTE: This test will FAIL in RED phase
      // @step And it should exit with code 1
      expect(exitCode).toBe(1);

      // And it should update .cursor/commands/fspec.md
      const updatedFspecMd = await readFile(
        join(testDir, '.cursor', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(updatedFspecMd).toContain('fspec --sync-version');
      expect(updatedFspecMd).not.toContain('0.5.0');

      // And it should update spec/CURSOR.md
      const updatedCursorMd = await readFile(
        join(testDir, 'spec', 'CURSOR.md'),
        'utf-8'
      );
      expect(updatedCursorMd).not.toBe('# Old CURSOR.md');

      // @step And it should print "⚠️  fspec files updated from v0.5.0 to v0.6.0" as plain text (no system-reminder tags)
      expect(output).toContain('⚠️');
      expect(output).toContain('fspec files updated from v0.5.0');
      expect(output).not.toContain('<system-reminder>'); // No tags for Cursor

      // And it should print Cursor-specific restart instructions
      expect(output).toContain('Restart Cursor');
      expect(output).toContain('/fspec again');
    });
  });

  describe('Scenario: Agent detection fallback when config missing', () => {
    it('should detect agent from filesystem and use generic restart message', async () => {
      // Given I have .claude/commands/fspec.md with old version
      await mkdir(join(testDir, '.claude', 'commands'), { recursive: true });
      await writeFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'fspec --sync-version 0.5.0\n\nOld content',
        'utf-8'
      );

      await writeFile(
        join(testDir, 'spec', 'CLAUDE.md'),
        '# Old CLAUDE.md',
        'utf-8'
      );

      // And spec/fspec-config.json is missing
      // (Don't create the file)

      // @step When I upgrade to fspec v0.6.0
      // @step And I run /fspec
      // @step And the AI agent executes "fspec --sync-version 0.5.0" as the first command
      let exitCode = 0;
      let output = '';
      try {
        output = execSync(`node ${fspecBin} --sync-version 0.5.0`, {
          cwd: testDir,
          encoding: 'utf-8',
          stdio: 'pipe',
        });
      } catch (error: any) {
        exitCode = error.status || 1;
        output = error.stdout || error.stderr || '';
      }

      // Then the command should detect version mismatch
      // NOTE: This test will FAIL in RED phase
      // @step And it should exit with code 1
      expect(exitCode).toBe(1);

      // And it should attempt to detect agent from filesystem (.claude/ exists)
      // And it should update both files
      const updatedFspecMd = await readFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );
      expect(updatedFspecMd).not.toContain('0.5.0');

      const updatedClaudeMd = await readFile(
        join(testDir, 'spec', 'CLAUDE.md'),
        'utf-8'
      );
      expect(updatedClaudeMd).not.toBe('# Old CLAUDE.md');

      // And it should detect Claude supports system-reminders
      expect(output).toContain('<system-reminder>');

      // And it should print generic restart instructions
      expect(output).toContain('Restart your AI agent');
      expect(output).toContain('/fspec again');
    });
  });

  describe('Scenario: fspec init embeds current version automatically', () => {
    it('should generate fspec.md with current version in --sync-version command', async () => {
      // Given I have fspec installed
      // (Using current installation)

      // When I run "fspec init --agent=claude"
      let output = '';
      try {
        output = execSync(`node ${fspecBin} init --agent=claude`, {
          cwd: testDir,
          encoding: 'utf-8',
          stdio: 'pipe',
        });
      } catch (error: any) {
        output = error.stdout || error.stderr || '';
      }

      // Then it should read version from package.json
      const packageJson = JSON.parse(
        await readFile(join(process.cwd(), 'package.json'), 'utf-8')
      );
      const currentVersion = packageJson.version;

      // And it should generate .claude/commands/fspec.md with current version
      const fspecMd = await readFile(
        join(testDir, '.claude', 'commands', 'fspec.md'),
        'utf-8'
      );

      // NOTE: This test will FAIL in RED phase - init doesn't embed version yet
      expect(fspecMd).toContain(`fspec --sync-version ${currentVersion}`);

      // And the version check command should appear before "IMMEDIATELY - run these commands" section
      const syncVersionIndex = fspecMd.indexOf('fspec --sync-version');
      const immediatelyIndex = fspecMd.indexOf(
        'IMMEDIATELY - run these commands'
      );
      expect(syncVersionIndex).toBeGreaterThan(0);
      expect(syncVersionIndex).toBeLessThan(immediatelyIndex);

      // And it should create spec/CLAUDE.md
      const claudeMdExists = existsSync(join(testDir, 'spec', 'CLAUDE.md'));
      expect(claudeMdExists).toBe(true);

      // And it should create spec/fspec-config.json with agent "claude"
      const config = JSON.parse(
        await readFile(join(testDir, 'spec', 'fspec-config.json'), 'utf-8')
      );
      expect(config.agent).toBe('claude');
    });
  });
});
