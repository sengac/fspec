/**
 * Feature: spec/features/unconfigured-research-tool-visibility-and-discovery.feature
 *
 * Tests for research tool configuration status detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { getToolConfigurationStatus, getConfigExample } from '../registry';
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

  describe('Scenario: List tools with no tools configured', () => {
    it('should detect AST as configured and others as unconfigured', async () => {
      // @step Given I have no research tools configured in spec/fspec-config.json
      // Empty config file (no research tools configured)
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      // @step When I run 'fspec research'
      // This tests the status detection that powers the listing
      const status = await getToolConfigurationStatus(testDir);

      // @step Then I should see only AST listed as available
      const astStatus = status.get('ast');
      expect(astStatus).toBeDefined();
      expect(astStatus!.configured).toBe(true);
      expect(astStatus!.reason).toContain('No configuration required');

      // @step And I should see a message about using --all to see unconfigured tools
      // Verify other tools are detected as unconfigured
      const perplexityStatus = status.get('perplexity');
      expect(perplexityStatus).toBeDefined();
      expect(perplexityStatus!.configured).toBe(false);
      expect(perplexityStatus!.requiredFields).toContain('apiKey');

      const jiraStatus = status.get('jira');
      expect(jiraStatus).toBeDefined();
      expect(jiraStatus!.configured).toBe(false);
      expect(jiraStatus!.requiredFields).toEqual(
        expect.arrayContaining(['jiraUrl', 'username', 'apiToken'])
      );
    });
  });

  describe('Scenario: List all tools including unconfigured ones', () => {
    it('should show all 5 tools with configuration status indicators', async () => {
      // @step Given I have no research tools configured except AST
      // Empty config (AST doesn't need config)
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      // @step When I run 'fspec research --all'
      const status = await getToolConfigurationStatus(testDir);

      // @step Then I should see all 5 research tools listed
      expect(status.size).toBe(5);
      expect(status.has('ast')).toBe(true);
      expect(status.has('perplexity')).toBe(true);
      expect(status.has('jira')).toBe(true);
      expect(status.has('confluence')).toBe(true);
      expect(status.has('stakeholder')).toBe(true);

      // @step And AST should show as configured with ✓ indicator
      const astStatus = status.get('ast');
      expect(astStatus!.configured).toBe(true);

      // @step And Perplexity should show as not configured with ✗ indicator
      const perplexityStatus = status.get('perplexity');
      expect(perplexityStatus!.configured).toBe(false);

      // @step And each unconfigured tool should show JSON config example
      expect(perplexityStatus!.configExample).toBeDefined();
      expect(perplexityStatus!.configExample).toContain('research');
      expect(perplexityStatus!.configExample).toContain('perplexity');
      expect(perplexityStatus!.configExample).toContain('apiKey');
    });

    it('should detect configured tools when config exists', async () => {
      // Setup: Configure Perplexity
      const config = {
        research: {
          perplexity: {
            apiKey: 'test-key-12345',
          },
        },
      };
      fs.writeFileSync(configPath, JSON.stringify(config), 'utf-8');

      const status = await getToolConfigurationStatus(testDir);

      // Verify Perplexity is now detected as configured
      const perplexityStatus = status.get('perplexity');
      expect(perplexityStatus!.configured).toBe(true);
      expect(perplexityStatus!.reason).toContain('configured');

      // AST still configured (no config needed)
      const astStatus = status.get('ast');
      expect(astStatus!.configured).toBe(true);

      // JIRA still unconfigured
      const jiraStatus = status.get('jira');
      expect(jiraStatus!.configured).toBe(false);
    });
  });

  describe('getConfigExample', () => {
    it('should return valid JSON for Perplexity', () => {
      // Test that config examples are valid JSON
      const example = getConfigExample('perplexity');
      expect(example).toBeDefined();

      const parsed = JSON.parse(example);
      expect(parsed.research.perplexity.apiKey).toBeDefined();
      expect(typeof parsed.research.perplexity.apiKey).toBe('string');
    });

    it('should return valid JSON for JIRA', () => {
      const example = getConfigExample('jira');
      expect(example).toBeDefined();

      const parsed = JSON.parse(example);
      expect(parsed.research.jira.jiraUrl).toBeDefined();
      expect(parsed.research.jira.username).toBeDefined();
      expect(parsed.research.jira.apiToken).toBeDefined();
    });

    it('should return valid JSON for Confluence', () => {
      const example = getConfigExample('confluence');
      expect(example).toBeDefined();

      const parsed = JSON.parse(example);
      expect(parsed.research.confluence.confluenceUrl).toBeDefined();
      expect(parsed.research.confluence.username).toBeDefined();
      expect(parsed.research.confluence.apiToken).toBeDefined();
    });

    it('should return valid JSON for Stakeholder', () => {
      const example = getConfigExample('stakeholder');
      expect(example).toBeDefined();

      const parsed = JSON.parse(example);
      expect(parsed.research.stakeholder).toBeDefined();
      // Stakeholder should show webhook options
      expect(
        parsed.research.stakeholder.teams || parsed.research.stakeholder.slack
      ).toBeDefined();
    });
  });
});
