/**
 * Feature: spec/features/blocklist-core-command-tool-filtering.feature
 *
 * Integration tests for blocklist NAPI bindings.
 * These tests verify the Rust blocklist module works correctly when called from TypeScript.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import {
  blocklistInit,
  blocklistLoad,
  blocklistSave,
  blocklistCheck,
  blocklistSystemPath,
  blocklistProjectPath,
  blocklistAllowSession,
  blocklistIsSessionAllowed,
  blocklistClearSessionAllowances,
} from '../../codelet/napi';
import type { JsBlocklistConfig, JsCheckResult } from '../../codelet/napi';

describe('Feature: Blocklist Core - Command/Tool Filtering', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'blocklist-test-'));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Block dangerous command with guidance', () => {
    // @step Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
    // @step When the AI runs "git checkout main" via Bash
    // @step Then the command should be blocked
    // @step And the AI should receive error "Blocked: git checkout is deprecated. Use git switch main instead."

    it('should block git checkout and provide guidance via NAPI', async () => {
      // Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          {
            id: 'git-checkout-block',
            pattern: '^git\\s+checkout\\b',
            action: 'block',
            reason: 'git checkout is deprecated',
            guidance: 'Use git switch instead',
          },
        ],
      };

      // Save config to project path
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);

      // Initialize blocklist from project
      blocklistInit(tmpDir);

      // When the AI runs "git checkout main" via Bash
      const result: JsCheckResult = blocklistCheck('git checkout main');

      // Then the command should be blocked
      expect(result.blocked).toBe(true);
      expect(result.allowed).toBe(false);

      // And the AI should receive reason and guidance
      expect(result.reason).toContain('git checkout is deprecated');
      expect(result.guidance).toContain('git switch');
      expect(result.matchedRuleId).toBe('git-checkout-block');
    });
  });

  describe('Scenario: Block Bash usage for file reading with tool guidance', () => {
    // @step Given a blocklist rule exists blocking "cat" commands with reason "Use Read tool instead"
    // @step When the AI runs "cat src/file.ts" via Bash
    // @step Then the command should be blocked
    // @step And the AI should receive error "Blocked: Use the Read tool for file reading, not Bash."

    it('should block cat command and suggest Read tool via NAPI', async () => {
      // Given a blocklist rule exists blocking "cat" commands
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          {
            id: 'cat-block',
            pattern: '^cat\\s+',
            action: 'block',
            reason: 'Use the Read tool for file reading, not Bash',
            guidance: 'This ensures proper encoding and line number display.',
          },
        ],
      };

      // Save and initialize
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);
      blocklistInit(tmpDir);

      // When the AI runs "cat src/file.ts" via Bash
      const result: JsCheckResult = blocklistCheck('cat src/file.ts');

      // Then the command should be blocked
      expect(result.blocked).toBe(true);
      expect(result.allowed).toBe(false);

      // And the AI should receive reason and guidance
      expect(result.reason).toContain('Use the Read tool');
      expect(result.guidance).toContain('proper encoding');
    });
  });

  describe('Scenario: Project config overrides system config', () => {
    // @step Given system blocklist at "~/.fspec/blocklist.json" blocks "rm -rf"
    // @step And project blocklist at ".fspec/blocklist.json" allows "rm -rf ./node_modules"
    // @step When the AI runs "rm -rf ./node_modules"
    // @step Then the command should be allowed
    // @step When the AI runs "rm -rf /"
    // @step Then the command should be blocked

    it('should allow project-specific overrides via NAPI', async () => {
      // Given project blocklist allows "rm -rf ./node_modules" but blocks "rm -rf" generally
      // Note: We're testing with project config only as system config requires home dir access
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          // Allow rule MUST come first (more specific)
          {
            id: 'rm-rf-node-modules-allow',
            pattern: '^rm\\s+-rf\\s+\\./node_modules\\b',
            action: 'allow',
            reason: '',
          },
          // Block rule comes second (less specific)
          {
            id: 'rm-rf-block',
            pattern: '^rm\\s+-rf\\b',
            action: 'block',
            reason: 'Dangerous command - rm -rf can delete everything',
          },
        ],
      };

      // Save and initialize
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);
      blocklistInit(tmpDir);

      // When the AI runs "rm -rf ./node_modules" - should be allowed
      const allowedResult: JsCheckResult = blocklistCheck(
        'rm -rf ./node_modules'
      );
      expect(allowedResult.allowed).toBe(true);
      expect(allowedResult.blocked).toBe(false);

      // When the AI runs "rm -rf /" - should be blocked
      const blockedResult: JsCheckResult = blocklistCheck('rm -rf /');
      expect(blockedResult.blocked).toBe(true);
      expect(blockedResult.allowed).toBe(false);
    });
  });

  describe('Scenario: Blocklist path functions work correctly', () => {
    it('should return correct system path', () => {
      const systemPath = blocklistSystemPath();
      // System path should be either null (no home) or contain .fspec/blocklist.json
      if (systemPath !== null) {
        expect(systemPath).toContain('.fspec');
        expect(systemPath).toContain('blocklist.json');
      }
    });

    it('should return correct project path', () => {
      const projectPath = blocklistProjectPath(tmpDir);
      expect(projectPath).toBe(join(tmpDir, '.fspec', 'blocklist.json'));
    });
  });

  describe('Scenario: Load and save config persistence', () => {
    it('should persist config to JSON file', async () => {
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          {
            id: 'test-rule',
            pattern: '^test\\s+',
            action: 'block',
            reason: 'Test reason',
            guidance: 'Test guidance',
          },
        ],
      };

      // Save config
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);

      // Verify file exists and contains correct JSON
      const filePath = join(tmpDir, '.fspec', 'blocklist.json');
      const fileContent = await readFile(filePath, 'utf-8');
      const parsedConfig = JSON.parse(fileContent);

      expect(parsedConfig.version).toBe('1.0.0');
      expect(parsedConfig.rules).toHaveLength(1);
      expect(parsedConfig.rules[0].id).toBe('test-rule');
      expect(parsedConfig.rules[0].action).toBe('block');
    });

    it('should load config from JSON file', async () => {
      // Create config file manually
      const configPath = join(tmpDir, '.fspec', 'blocklist.json');
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      await writeFile(
        configPath,
        JSON.stringify({
          version: '2.0.0',
          rules: [
            {
              id: 'loaded-rule',
              pattern: '^loaded',
              action: 'block',
              reason: 'Loaded reason',
            },
          ],
        })
      );

      // Load config
      const loadedConfig = blocklistLoad(tmpDir);

      // Note: blocklistLoad merges system + project rules, so we check for presence
      expect(loadedConfig.version).toBe('2.0.0');
      expect(loadedConfig.rules.length).toBeGreaterThanOrEqual(1);
      expect(
        loadedConfig.rules.find(r => r.id === 'loaded-rule')
      ).toBeDefined();
      expect(
        loadedConfig.rules.find(r => r.id === 'loaded-rule')?.pattern
      ).toBe('^loaded');
    });
  });

  describe('Scenario: Allow unmatched commands', () => {
    it('should allow commands that do not match any rule', async () => {
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          {
            id: 'git-checkout-block',
            pattern: '^git\\s+checkout\\b',
            action: 'block',
            reason: 'Use git switch',
          },
        ],
      };

      // Save and initialize
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);
      blocklistInit(tmpDir);

      // Unmatched commands should be allowed
      const npmResult = blocklistCheck('npm install');
      expect(npmResult.allowed).toBe(true);
      expect(npmResult.blocked).toBe(false);

      const gitStatusResult = blocklistCheck('git status');
      expect(gitStatusResult.allowed).toBe(true);
      expect(gitStatusResult.blocked).toBe(false);

      const gitSwitchResult = blocklistCheck('git switch main');
      expect(gitSwitchResult.allowed).toBe(true);
      expect(gitSwitchResult.blocked).toBe(false);
    });
  });

  describe('Scenario: Prompt action type', () => {
    it('should return prompt status for prompt action rules', async () => {
      const config: JsBlocklistConfig = {
        version: '1.0.0',
        rules: [
          {
            id: 'sensitive-path-prompt',
            pattern: '/etc/',
            action: 'prompt',
            reason: 'Accessing system configuration files',
            guidance: 'Confirm you want to access /etc/ directory',
          },
        ],
      };

      // Save and initialize
      await mkdir(join(tmpDir, '.fspec'), { recursive: true });
      blocklistSave(tmpDir, config);
      blocklistInit(tmpDir);

      const result = blocklistCheck('cat /etc/passwd');
      // Prompt action: not blocked (needs confirmation), not allowed (needs confirmation)
      expect(result.blocked).toBe(false);
      expect(result.allowed).toBe(false);
      expect(result.reason).toContain('system configuration');
      expect(result.matchedRuleId).toBe('sensitive-path-prompt');
    });
  });
});

/**
 * Feature: spec/features/sensitive-path-prompts.feature
 *
 * Integration tests for session allowance NAPI bindings (BLOCK-005).
 * These tests verify the Rust session allowance module works correctly when called from TypeScript.
 */
describe('Feature: Sensitive Path Prompts - Session Allowances NAPI Integration', () => {
  beforeEach(() => {
    // Always start with a clean session state
    blocklistClearSessionAllowances();
  });

  afterEach(() => {
    // Clean up after each test
    blocklistClearSessionAllowances();
  });

  describe('Scenario: Prompt for SSH config access - user allows for session', () => {
    // @step Given a blocklist rule exists prompting for "~/.ssh" access
    // @step When the AI tries to read "~/.ssh/config"
    // @step And the user selects "Allow Session"
    // @step Then the file should be read successfully
    // @step When the AI tries to read "~/.ssh/known_hosts" later in the same session
    // @step Then the file should be read without prompting

    it('should allow session and remember pattern via NAPI', () => {
      const pattern = '~/.ssh';

      // Initially, pattern should not be allowed
      expect(blocklistIsSessionAllowed(pattern)).toBe(false);

      // User selects "Allow Session"
      blocklistAllowSession(pattern);

      // Pattern is now allowed for session
      expect(blocklistIsSessionAllowed(pattern)).toBe(true);

      // Same pattern should still be allowed (no re-prompt needed)
      expect(blocklistIsSessionAllowed(pattern)).toBe(true);
    });
  });

  describe('Scenario: Session allowances cleared on TUI restart', () => {
    // @step Given a blocklist rule prompts for "npm install" commands
    // @step When the AI runs "npm install" and user allows for session
    // @step Then the AI can run "npm install lodash" without prompting
    // @step When the user exits and restarts the TUI
    // @step And the AI runs "npm install axios"
    // @step Then the user should be prompted again

    it('should clear session allowances when clearSessionAllowances is called (simulating TUI restart)', () => {
      const pattern = '^npm\\s+install';

      // User allows for session
      blocklistAllowSession(pattern);
      expect(blocklistIsSessionAllowed(pattern)).toBe(true);

      // TUI restart simulation - clear all session allowances
      blocklistClearSessionAllowances();

      // Pattern should no longer be allowed (user should be prompted again)
      expect(blocklistIsSessionAllowed(pattern)).toBe(false);
    });
  });

  describe('Scenario: Multiple patterns can be allowed in session', () => {
    it('should track multiple patterns independently via NAPI', () => {
      const sshPattern = '~/.ssh';
      const envPattern = '.env';
      const fspecPattern = '~/.fspec';

      // Allow multiple patterns
      blocklistAllowSession(sshPattern);
      blocklistAllowSession(envPattern);
      blocklistAllowSession(fspecPattern);

      // All should be allowed
      expect(blocklistIsSessionAllowed(sshPattern)).toBe(true);
      expect(blocklistIsSessionAllowed(envPattern)).toBe(true);
      expect(blocklistIsSessionAllowed(fspecPattern)).toBe(true);

      // Different pattern should NOT be allowed
      expect(blocklistIsSessionAllowed('~/.aws')).toBe(false);
    });
  });

  describe('Scenario: Session allowances use exact pattern matching', () => {
    it('should require exact pattern match via NAPI', () => {
      // Allow a specific pattern
      blocklistAllowSession('~/.ssh');

      // Exact match works
      expect(blocklistIsSessionAllowed('~/.ssh')).toBe(true);

      // Partial matches do NOT work (not substring matching)
      expect(blocklistIsSessionAllowed('~/.ss')).toBe(false);
      expect(blocklistIsSessionAllowed('~/.sshkeys')).toBe(false);
      expect(blocklistIsSessionAllowed('.ssh')).toBe(false);
    });
  });

  describe('Scenario: Allow Once does not persist', () => {
    // @step Given the user selects "Allow Once"
    // @step Then the file should be read successfully
    // @step And subsequent access to the same file should prompt again

    it('should not add to session allowances when user selects Allow Once (verified by not calling blocklistAllowSession)', () => {
      const pattern = '~/.ssh/config';

      // Allow Once means we DON'T call blocklistAllowSession
      // Pattern should not be in session allowances
      expect(blocklistIsSessionAllowed(pattern)).toBe(false);

      // Even after checking, still not allowed
      expect(blocklistIsSessionAllowed(pattern)).toBe(false);
    });
  });
});
