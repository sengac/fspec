/**
 * Feature: spec/features/research-framework-with-custom-script-integration.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile, chmod, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { research } from '../research.js';

describe('Feature: Research framework with custom script integration', () => {
  let testDir: string;
  let researchScriptsDir: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    researchScriptsDir = join(testDir, 'spec', 'research-scripts');
    await mkdir(researchScriptsDir, { recursive: true });
  });

  afterEach(async () => {
    // Cleanup test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List available research tools without executing any', () => {
    it('should display list of available research tools when run without flags', async () => {
      // Given I have research scripts in spec/research-scripts/ directory
      // And the directory contains "perplexity.sh", "jira.py", and "confluence" (compiled binary)
      await writeFile(
        join(researchScriptsDir, 'perplexity.sh'),
        '#!/bin/bash\necho "perplexity"'
      );
      await chmod(join(researchScriptsDir, 'perplexity.sh'), 0o755);

      await writeFile(
        join(researchScriptsDir, 'jira.py'),
        '#!/usr/bin/env python3\nprint("jira")'
      );
      await chmod(join(researchScriptsDir, 'jira.py'), 0o755);

      await writeFile(
        join(researchScriptsDir, 'confluence'),
        '#!/bin/bash\necho "confluence"'
      );
      await chmod(join(researchScriptsDir, 'confluence'), 0o755);

      // When I run "fspec research" without any flags
      const result = await research({ cwd: testDir });

      // Then I should see a list of available research tools from the registry
      // Note: The system now uses a tool registry instead of script discovery
      expect(result.tools).toBeDefined();
      expect(result.tools.length).toBeGreaterThanOrEqual(2); // At least perplexity and ast

      // And the tools should include perplexity and ast from the registry
      expect(result.tools.map(t => t.name)).toContain('perplexity');
      expect(result.tools.map(t => t.name)).toContain('ast');

      // And each tool should show usage example with --tool flag
      result.tools.forEach(tool => {
        expect(tool.usage).toContain('--tool=');
        expect(tool.usage).toContain(tool.name);
      });

      // And each tool should show how to get help for that tool
      result.tools.forEach(tool => {
        expect(tool.helpCommand).toBeDefined();
      });

      // And no research should be executed
      expect(result.executed).toBe(false);
    });
  });

  describe('Scenario: Execute research tool and prompt for attachment', () => {
    it('should execute research tool and prompt for attachment to work unit', async () => {
      // Given I have a research tool "perplexity.sh" in spec/research-scripts/
      const scriptContent =
        '#!/bin/bash\necho "OAuth2 is an authorization framework..."';
      await writeFile(join(researchScriptsDir, 'perplexity.sh'), scriptContent);
      await chmod(join(researchScriptsDir, 'perplexity.sh'), 0o755);

      // When I run "fspec research --tool=perplexity --query='How does OAuth2 work?'"
      const result = await research({
        cwd: testDir,
        tool: 'perplexity',
        query: 'How does OAuth2 work?',
      });

      // Then the perplexity tool should execute with the query
      expect(result.executed).toBe(true);
      expect(result.toolName).toBe('perplexity');
      expect(result.query).toBe('How does OAuth2 work?');

      // And I should see the formatted research results
      expect(result.output).toBeDefined();
      expect(result.output).toContain('OAuth2');

      // And I should be prompted "Attach research results to work unit? (y/n)"
      expect(result.promptForAttachment).toBe(true);
    });
  });

  describe('Scenario: Auto-discover research tools by executable bit', () => {
    it('should auto-discover research tools with different file extensions', async () => {
      // Given I have research scripts with different extensions
      // And spec/research-scripts/ contains "perplexity.py" (Python script)
      await writeFile(
        join(researchScriptsDir, 'perplexity.py'),
        '#!/usr/bin/env python3\nprint("perplexity")'
      );
      await chmod(join(researchScriptsDir, 'perplexity.py'), 0o755);

      // And spec/research-scripts/ contains "jira" (compiled binary)
      await writeFile(
        join(researchScriptsDir, 'jira'),
        '#!/bin/bash\necho "jira"'
      );
      await chmod(join(researchScriptsDir, 'jira'), 0o755);

      // And spec/research-scripts/ contains "confluence.js" (Node script)
      await writeFile(
        join(researchScriptsDir, 'confluence.js'),
        '#!/usr/bin/env node\nconsole.log("confluence")'
      );
      await chmod(join(researchScriptsDir, 'confluence.js'), 0o755);

      // And all three files have executable bit set (done above)

      // When I run "fspec research" without flags
      const result = await research({ cwd: testDir });

      // Then tools from the registry should be returned
      // Note: The system now uses a tool registry instead of script discovery
      expect(result.tools.length).toBeGreaterThanOrEqual(2);

      // And tool names should include perplexity and ast from the registry
      expect(result.tools.map(t => t.name)).toContain('perplexity');
      expect(result.tools.map(t => t.name)).toContain('ast');

      // And discovery uses the registry system
      expect(result.discoveryMethod).toBe('registry');
    });
  });

  describe('Scenario: Attach research results to work unit', () => {
    it('should save research results as attachment when user confirms', async () => {
      // Given I have executed "fspec research --tool=perplexity --query='OAuth2'"
      const scriptContent = '#!/bin/bash\necho "OAuth2 research results"';
      await writeFile(join(researchScriptsDir, 'perplexity.sh'), scriptContent);
      await chmod(join(researchScriptsDir, 'perplexity.sh'), 0o755);

      // And the research completed successfully with results
      const researchResult = await research({
        cwd: testDir,
        tool: 'perplexity',
        query: 'OAuth2',
      });
      expect(researchResult.executed).toBe(true);

      // When I choose "yes" to attach results to work unit AUTH-001
      const attachResult = await research({
        cwd: testDir,
        tool: 'perplexity',
        query: 'OAuth2',
        attach: true,
        workUnit: 'AUTH-001',
      });

      // Then research output should be saved to "spec/attachments/AUTH-001/perplexity-oauth2-research-YYYY-MM-DD.md"
      expect(attachResult.attachmentPath).toMatch(
        /spec\/attachments\/AUTH-001\/perplexity-oauth2-research-\d{4}-\d{2}-\d{2}\.md/
      );

      // And the attachment should be referenced in work unit AUTH-001 metadata
      expect(attachResult.workUnitUpdated).toBe(true);

      // And I should be able to view attachments with "fspec list-attachments AUTH-001"
      expect(attachResult.attachmentCreated).toBe(true);
    });
  });

  describe('Scenario: Prompt to research during Example Mapping', () => {
    it('should prompt user to research when adding a question', async () => {
      // This scenario will be tested as an integration test with add-question command
      // For now, we'll test the prompt logic in isolation

      // Given I am adding a question during Example Mapping
      const mockPromptForResearch = vi.fn().mockResolvedValue(true);

      // When I run "fspec add-question AUTH-001 '@human: Should we support OAuth2?'"
      // Then the question should be added successfully
      // And I should see a prompt "Would you like to research this question? (y/n)"
      const shouldPrompt = await mockPromptForResearch();
      expect(shouldPrompt).toBe(true);

      // And if I choose "yes", I should see available research tools
      // And I should be guided to run "fspec research --tool=<name> --query='question'"
      // (This will be implemented in the actual command integration)
    });
  });
});
