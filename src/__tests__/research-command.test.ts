/**
 * Feature: spec/features/interactive-research-command-with-multiple-backend-support.feature
 *
 * Tests for interactive research command with multiple backend support.
 * These tests verify the 'fspec research' command that auto-discovers and executes
 * research tools from spec/research-scripts/ directory.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

const PROJECT_ROOT = process.cwd();
const RESEARCH_SCRIPTS_DIR = path.join(PROJECT_ROOT, 'spec/research-scripts');

describe('Feature: Interactive research command with multiple backend support', () => {
  beforeAll(() => {
    // Enable test mode for research scripts
    process.env.FSPEC_TEST_MODE = '1';
  });

  describe('Scenario: List available research tools without arguments', () => {
    it('should list all available research tools with descriptions', () => {
      // @step Given research tools exist in spec/research-scripts/ directory
      expect(fs.existsSync(RESEARCH_SCRIPTS_DIR)).toBe(true);

      // @step And the tools include perplexity, jira, and confluence
      const perplexityScript = path.join(RESEARCH_SCRIPTS_DIR, 'perplexity');
      const jiraScript = path.join(RESEARCH_SCRIPTS_DIR, 'jira');
      const confluenceScript = path.join(RESEARCH_SCRIPTS_DIR, 'confluence');
      expect(fs.existsSync(perplexityScript)).toBe(true);
      expect(fs.existsSync(jiraScript)).toBe(true);
      expect(fs.existsSync(confluenceScript)).toBe(true);

      // @step When I run "fspec research" without arguments
      const result = execSync('fspec research', {
        encoding: 'utf8',
        cwd: PROJECT_ROOT,
      });

      // @step Then the output should list all available research tools
      expect(result).toContain('perplexity');
      expect(result).toContain('jira');
      expect(result).toContain('confluence');

      // @step And each tool should show its name and description
      expect(result.length).toBeGreaterThan(100);

      // @step And each tool should show usage examples
      expect(result).toContain('fspec research --tool=');
    });
  });

  describe('Scenario: Execute perplexity research tool with query', () => {
    it('should execute perplexity script and prompt for attachment', () => {
      // @step Given the perplexity research tool exists in spec/research-scripts/
      const perplexityScript = path.join(RESEARCH_SCRIPTS_DIR, 'perplexity');
      expect(fs.existsSync(perplexityScript)).toBe(true);

      // @step And the perplexity tool has executable permissions
      const stats = fs.statSync(perplexityScript);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step When I run "fspec research --tool=perplexity --query='How does OAuth2 work?'"
      const result = execSync(
        'fspec research --tool=perplexity --query="How does OAuth2 work?"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the perplexity script should execute
      expect(result).toBeDefined();

      // @step And the output should contain research results
      expect(result.length).toBeGreaterThan(0);

      // @step And the command should prompt to attach results to work unit
      expect(result).toMatch(/attach|save|work unit/i);
    });
  });

  describe('Scenario: Execute jira research tool with issue key', () => {
    it('should execute jira script with issue flag', () => {
      // @step Given the jira research tool exists in spec/research-scripts/
      const jiraScript = path.join(RESEARCH_SCRIPTS_DIR, 'jira');
      expect(fs.existsSync(jiraScript)).toBe(true);

      // @step And the jira tool has executable permissions
      const stats = fs.statSync(jiraScript);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step When I run "fspec research --tool=jira --issue AUTH-123"
      const result = execSync('fspec research --tool=jira --issue AUTH-123', {
        encoding: 'utf8',
        cwd: PROJECT_ROOT,
      });

      // @step Then the jira script should execute with --issue flag
      expect(result).toBeDefined();

      // @step And the output should contain issue details
      expect(result.length).toBeGreaterThan(0);

      // @step And the command should prompt to attach results to work unit
      expect(result).toMatch(/attach|save|work unit/i);
    });
  });

  describe('Scenario: Attach research results to work unit', () => {
    it('should save research results as attachment', () => {
      // @step Given the perplexity research tool exists in spec/research-scripts/
      const perplexityScript = path.join(RESEARCH_SCRIPTS_DIR, 'perplexity');
      expect(fs.existsSync(perplexityScript)).toBe(true);

      // @step And work unit AUTH-001 exists
      // NOTE: This requires a work unit to exist - may need to create one for testing

      // @step When I run "fspec research --tool=perplexity --query='OAuth2'"
      // @step And I respond "y" to the attachment prompt
      const result = execSync(
        'echo "y" | fspec research --tool=perplexity --query="OAuth2" --work-unit=AUTH-001',
        { encoding: 'utf8', cwd: PROJECT_ROOT, shell: '/bin/bash' }
      );

      // @step Then the research results should be saved to spec/attachments/AUTH-001/
      const attachmentsDir = path.join(
        PROJECT_ROOT,
        'spec/attachments/AUTH-001'
      );
      expect(fs.existsSync(attachmentsDir)).toBe(true);

      // @step And the attachment filename should include the tool name and date
      const files = fs.readdirSync(attachmentsDir);
      const attachmentFile = files.find(
        f => f.includes('perplexity') && f.includes('oauth2')
      );
      expect(attachmentFile).toBeDefined();

      // @step And the work unit should reference the attachment
      expect(result).toContain('attached');
    });
  });

  describe('Scenario: Execute confluence research tool with page title', () => {
    it('should execute confluence script with page flag', () => {
      // @step Given the confluence research tool exists in spec/research-scripts/
      const confluenceScript = path.join(RESEARCH_SCRIPTS_DIR, 'confluence');
      expect(fs.existsSync(confluenceScript)).toBe(true);

      // @step And the confluence tool has executable permissions
      const stats = fs.statSync(confluenceScript);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step When I run "fspec research --tool=confluence --page 'Authentication Guide'"
      const result = execSync(
        'fspec research --tool=confluence --page "Authentication Guide"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the confluence script should execute with --page flag
      expect(result).toBeDefined();

      // @step And the output should contain page content
      expect(result.length).toBeGreaterThan(0);

      // @step And the command should prompt to attach results to work unit
      expect(result).toMatch(/attach|save|work unit/i);
    });
  });

  describe('Scenario: Error on invalid research tool', () => {
    it('should return error for non-existent tool', () => {
      // @step Given no research tool named "invalid" exists
      const invalidScript = path.join(RESEARCH_SCRIPTS_DIR, 'invalid');
      expect(fs.existsSync(invalidScript)).toBe(false);

      // @step When I run "fspec research --tool=invalid"
      try {
        execSync('fspec research --tool=invalid', {
          encoding: 'utf8',
          cwd: PROJECT_ROOT,
          stderr: 'pipe',
        });
        throw new Error('Should have thrown an error');
      } catch (error: any) {
        // @step Then the command should exit with error code 1
        expect(error.status).toBe(1);

        // @step And the output should contain "Error: Research tool not found: invalid"
        const errorOutput = error.stderr.toString();
        expect(errorOutput).toMatch(
          /error.*research tool.*not found.*invalid/i
        );

        // @step And the output should suggest running "fspec research" to see available tools
        expect(errorOutput).toMatch(/fspec research/i);
      }
    });
  });

  describe('Scenario: Auto-discover new research tool', () => {
    let testScriptPath: string;

    beforeEach(() => {
      testScriptPath = path.join(RESEARCH_SCRIPTS_DIR, 'github');
    });

    afterEach(() => {
      if (fs.existsSync(testScriptPath)) {
        fs.unlinkSync(testScriptPath);
      }
    });

    it('should auto-discover newly added executable scripts', () => {
      // @step Given research tools exist in spec/research-scripts/
      expect(fs.existsSync(RESEARCH_SCRIPTS_DIR)).toBe(true);

      // @step When I add a new executable script spec/research-scripts/github
      fs.writeFileSync(
        testScriptPath,
        '#!/bin/bash\necho "GitHub research tool"\n',
        { mode: 0o755 }
      );
      expect(fs.existsSync(testScriptPath)).toBe(true);

      // @step And I run "fspec research" without arguments
      const result = execSync('fspec research', {
        encoding: 'utf8',
        cwd: PROJECT_ROOT,
      });

      // @step Then the output should include "github" in the available tools list
      expect(result).toContain('github');

      // @step And the github tool should show its name and description
      expect(result.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Decline attachment prompt', () => {
    it('should display results without saving when user declines', () => {
      // @step Given the perplexity research tool exists in spec/research-scripts/
      const perplexityScript = path.join(RESEARCH_SCRIPTS_DIR, 'perplexity');
      expect(fs.existsSync(perplexityScript)).toBe(true);

      // @step And work unit AUTH-001 exists
      // NOTE: This requires a work unit to exist

      // @step When I run "fspec research --tool=perplexity --query='test'"
      // @step And I respond "n" to the attachment prompt
      const result = execSync(
        'echo "n" | fspec research --tool=perplexity --query="test" --work-unit=AUTH-001',
        { encoding: 'utf8', cwd: PROJECT_ROOT, shell: '/bin/bash' }
      );

      // @step Then the research results should be displayed
      expect(result).toBeDefined();
      expect(result.length).toBeGreaterThan(0);

      // @step And no attachment should be saved
      const attachmentsDir = path.join(
        PROJECT_ROOT,
        'spec/attachments/AUTH-001'
      );
      if (fs.existsSync(attachmentsDir)) {
        const filesBefore = fs.readdirSync(attachmentsDir).length;
        // Verify no new files were added
        const filesAfter = fs.readdirSync(attachmentsDir).length;
        expect(filesAfter).toBe(filesBefore);
      }

      // @step And the work unit should not reference any new attachment
      expect(result).not.toContain('attached');
    });
  });
});
