/**
 * Feature: spec/features/unconfigured-research-tool-visibility-and-discovery.feature
 *
 * Tests for system-reminder display of research tools to AI agents
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { specifyingStateReminder } from '../system-reminder';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('Feature: Unconfigured research tool visibility and discovery', () => {
  let testDir: string;
  let configPath: string;

  beforeEach(() => {
    // Create temporary test directory
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-test-'));
    configPath = path.join(testDir, 'spec', 'fspec-config.json');

    // Ensure spec directory exists
    fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });
  });

  afterEach(() => {
    // Cleanup test directory
    if (fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: System-reminder shows all tools to AI agents', () => {
    it('should display all tools with configuration status in system-reminder', async () => {
      // @step Given I am an AI agent
      // AI agents receive system-reminders (this test simulates that)

      // @step And some research tools are not configured
      // Configure only Perplexity, leave others unconfigured
      const config = {
        research: {
          perplexity: {
            apiKey: 'test-key',
          },
        },
      };
      fs.writeFileSync(configPath, JSON.stringify(config), 'utf-8');

      const workUnit = {
        id: 'TEST-001',
        title: 'Test Work Unit',
        status: 'specifying' as const,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [
          {
            state: 'specifying' as const,
            timestamp: new Date().toISOString(),
          },
        ],
      };

      // @step When a system-reminder about research tools is displayed
      const reminder = await specifyingStateReminder(
        'TEST-001',
        workUnit,
        testDir
      );

      // @step Then I should see all 5 tools with configuration status
      expect(reminder).toContain('ast');
      expect(reminder).toContain('perplexity');
      expect(reminder).toContain('jira');
      expect(reminder).toContain('confluence');
      expect(reminder).toContain('stakeholder');

      // Verify status indicators
      expect(reminder).toMatch(/✓.*ast/);
      expect(reminder).toMatch(/✓.*perplexity/);
      expect(reminder).toMatch(/✗.*jira/);
      expect(reminder).toMatch(/✗.*confluence/);
      expect(reminder).toMatch(/✗.*stakeholder/);

      // @step And each unconfigured tool should show JSON config structure
      expect(reminder).toContain('"research"');
      expect(reminder).toContain('"jira"');
      expect(reminder).toContain('jiraUrl');

      // @step And config file paths should be mentioned
      expect(reminder).toContain('spec/fspec-config.json');
      expect(reminder).toContain('~/.fspec/fspec-config.json');
    });

    it('should show concise setup hints for AI agents in system-reminder', async () => {
      // Empty config - all tools except AST unconfigured
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      const workUnit = {
        id: 'TEST-002',
        title: 'Test Work Unit 2',
        status: 'specifying' as const,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        stateHistory: [
          {
            state: 'specifying' as const,
            timestamp: new Date().toISOString(),
          },
        ],
      };

      const reminder = await specifyingStateReminder(
        'TEST-002',
        workUnit,
        testDir
      );

      // System-reminder should be concise (AI can run --help for full details)
      expect(reminder).toContain('research');

      // Should mention how to get full help
      expect(reminder).toMatch(/--help|For full help/i);
    });
  });
});
