/**
 * Feature: spec/features/smart-research-integration-and-auto-attachment.feature
 *
 * Tests for smart research integration with AI-assisted extraction (RES-013).
 * Tests auto-attachment of research results and AI extraction of rules/examples.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import fs from 'fs/promises';
import path from 'path';
import os from 'os';
import {
  executeResearchWithPrompt,
  analyzeResearchOutput,
  acceptAISuggestions,
  executeResearchWithAutoAttach,
  extractRules,
  confirmAndAddRule,
} from '../research-integration';

describe('Feature: Smart Research Integration and Auto-Attachment', () => {
  let tempDir: string;
  let attachmentsDir: string;

  beforeEach(async () => {
    // Create temp directory for test
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'fspec-research-test-'));
    attachmentsDir = path.join(tempDir, 'spec', 'attachments');
    await fs.mkdir(attachmentsDir, { recursive: true });
  });

  afterEach(async () => {
    // Clean up temp directory
    if (tempDir) {
      await fs.rm(tempDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Prompt to save research results as attachment after research tool execution', () => {
    it('should prompt to save research results after execution', async () => {
      // @step Given I have a work unit AUTH-001 in specifying state
      const workUnitId = 'AUTH-001';
      const workUnitState = 'specifying';

      // @step When I run "fspec research --tool=perplexity --query="OAuth best practices" --work-unit=AUTH-001"
      const result = await executeResearchWithPrompt(
        'perplexity',
        'OAuth best practices',
        workUnitId
      );

      // @step And the research tool returns results
      expect(result).toBeDefined();
      expect(result.output).toBeDefined();

      // @step Then the system should prompt "Save research results as attachment? (y/n)"
      expect(result.prompt).toBe('Save research results as attachment? (y/n)');

      // @step And the prompt should wait for user input
      expect(result.waitingForInput).toBe(true);
    });
  });

  describe('Scenario: AI analyzes research output and suggests extractable rules and examples', () => {
    it('should analyze research output and suggest rules/examples', async () => {
      // @step Given I have saved research results for AUTH-001
      const workUnitId = 'AUTH-001';
      const researchOutput =
        'OAuth best practices: Use PKCE for mobile apps. JWT tokens should expire within 24 hours. Always validate redirect URIs.';

      // @step When the AI analyzes the research output
      const analysis = await analyzeResearchOutput(researchOutput, workUnitId);

      // @step Then the system should display "Found 3 rules, 5 examples"
      expect(analysis).toBeDefined();
      expect(analysis.summary).toMatch(/Found \d+ rules?, \d+ examples?/);

      // @step And the system should prompt "Add to AUTH-001? (y/n/edit)"
      expect(analysis.prompt).toBe('Add to AUTH-001? (y/n/edit)');
    });
  });

  describe('Scenario: Accept AI suggestions and add to Example Map with attachment saved', () => {
    it('should save rules/examples to Example Map and save attachments', async () => {
      // @step Given AI has suggested 3 rules and 5 examples for AUTH-001
      const workUnitId = 'AUTH-001';
      const aiSuggestions = {
        rules: [
          'Use PKCE for mobile apps',
          'JWT tokens must expire within 24 hours',
          'Always validate redirect URIs',
        ],
        examples: [
          'Example 1',
          'Example 2',
          'Example 3',
          'Example 4',
          'Example 5',
        ],
      };
      const rawOutput = 'OAuth best practices research output...';

      // @step When I accept the AI suggestions
      const result = await acceptAISuggestions(
        workUnitId,
        aiSuggestions,
        rawOutput,
        tempDir
      );

      // @step Then the rules and examples should be added to AUTH-001 Example Map
      expect(result).toBeDefined();
      expect(result.addedToExampleMap).toBe(true);
      expect(result.rulesAdded).toBe(3);
      expect(result.examplesAdded).toBe(5);

      // @step And the raw output should be saved to spec/attachments/AUTH-001/perplexity-oauth-research.md
      const rawFilePath = path.join(
        attachmentsDir,
        workUnitId,
        'perplexity-oauth-research.md'
      );
      expect(result.rawAttachmentPath).toContain(
        'perplexity-oauth-research.md'
      );

      // @step And the structured extraction should be saved to spec/attachments/AUTH-001/perplexity-oauth-research-extracted.json
      const extractedFilePath = path.join(
        attachmentsDir,
        workUnitId,
        'perplexity-oauth-research-extracted.json'
      );
      expect(result.extractedAttachmentPath).toContain(
        'perplexity-oauth-research-extracted.json'
      );
    });
  });

  describe('Scenario: Auto-attach research results without prompts using --auto-attach flag', () => {
    it('should automatically save research results without prompts', async () => {
      // @step Given I have a work unit AUTH-001 in specifying state
      const workUnitId = 'AUTH-001';
      const workUnitState = 'specifying';

      // @step When I run "fspec research --tool=ast --query="authentication patterns" --work-unit=AUTH-001 --auto-attach"
      const result = await executeResearchWithAutoAttach(
        'ast',
        'authentication patterns',
        workUnitId,
        tempDir
      );

      // @step Then the research results should be automatically saved without prompts
      expect(result).toBeDefined();
      expect(result.prompted).toBe(false);
      expect(result.attachmentSaved).toBe(true);

      // @step And the attachment should be saved to spec/attachments/AUTH-001/
      expect(result.attachmentPath).toContain(
        `spec/attachments/${workUnitId}/`
      );
    });
  });

  describe('Scenario: AI extracts specific business rule from research output', () => {
    it('should extract specific business rule and present for confirmation', async () => {
      // @step Given I have research output about JWT token expiration
      const researchOutput =
        'For security, JWT tokens should expire within 24 hours. Refresh tokens can last longer but must be rotated.';

      // @step When the AI analyzes the content
      const extraction = await extractRules(researchOutput);

      // @step Then the AI should extract rule "JWT tokens must expire within 24 hours"
      expect(extraction).toBeDefined();
      expect(extraction.rules).toBeDefined();
      expect(extraction.rules.length).toBeGreaterThan(0);
      expect(extraction.rules[0]).toContain('JWT tokens');
      expect(extraction.rules[0]).toContain('24 hours');

      // @step And the extracted rule should be presented for user confirmation
      expect(extraction.requiresConfirmation).toBe(true);
      expect(extraction.confirmationPrompt).toBeDefined();

      // @step And after confirmation the rule should be added to the Example Map
      // This is tested by accepting the confirmation
      const confirmed = await confirmAndAddRule(
        extraction.rules[0],
        'AUTH-001'
      );
      expect(confirmed).toBe(true);
    });
  });
});
