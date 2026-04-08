/**
 * Feature: spec/features/fail-fast-foundation-workflow-for-weaker-llms.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * Covers all 14 scenarios of FOUND-044 (projectType length rules, problemImpact
 * enum, draft observability, discover error formatting, list sections, help text).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile, access, stat } from 'fs/promises';
import { join } from 'path';
import { updateFoundation } from '../update-foundation';
import { showFoundation } from '../show-foundation';
import { discoverFoundation } from '../discover-foundation';
import { listFoundationSections } from '../list-foundation-sections';
import updateFoundationHelpConfig from '../update-foundation-help';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

async function fileExists(path: string): Promise<boolean> {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * Create a draft with unfilled placeholder fields, projectType being a
 * [DETECTED: ...] placeholder so the write under test starts from a
 * fresh discovery state.
 */
function makeDraftWithProjectTypePlaceholder(): object {
  return {
    version: '2.0.0',
    project: {
      name: 'TestProject',
      vision: 'A test project for fail-fast validation',
      projectType: '[DETECTED: cli-tool]',
    },
    problemSpace: {
      primaryProblem: {
        title: '[QUESTION: What problem?]',
        description: '[QUESTION: Describe the problem]',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: '[QUESTION: What can users DO?]',
      capabilities: [],
    },
    personas: [
      {
        name: '[QUESTION: Who uses this?]',
        description: '[QUESTION: Who uses this?]',
        goals: ['[QUESTION: What are their goals?]'],
      },
    ],
  };
}

/**
 * Render a CommandHelpConfig object as the help text a user would see.
 * Concatenates every user-facing text field so assertions can run without
 * spawning a child CLI.
 */
function renderHelpConfig(config: unknown): string {
  if (!config || typeof config !== 'object') {
    return '';
  }
  const cfg = config as Record<string, unknown>;
  const parts: string[] = [];
  if (typeof cfg.description === 'string') {
    parts.push(cfg.description);
  }
  if (typeof cfg.usage === 'string') {
    parts.push(cfg.usage);
  }
  if (Array.isArray(cfg.examples)) {
    for (const example of cfg.examples) {
      if (typeof example === 'string') {
        parts.push(example);
      } else if (example && typeof example === 'object') {
        parts.push(JSON.stringify(example));
      }
    }
  }
  if (Array.isArray(cfg.notes)) {
    for (const note of cfg.notes) {
      if (typeof note === 'string') {
        parts.push(note);
      }
    }
  }
  if (Array.isArray(cfg.sections)) {
    for (const section of cfg.sections) {
      if (section && typeof section === 'object') {
        parts.push(JSON.stringify(section));
      }
    }
  }
  if (Array.isArray(cfg.arguments)) {
    for (const arg of cfg.arguments) {
      if (arg && typeof arg === 'object') {
        parts.push(JSON.stringify(arg));
      }
    }
  }
  return parts.join('\n');
}

describe('Feature: Fail-Fast Foundation Workflow for Weaker LLMs', () => {
  let testDir: string;
  let draftPath: string;
  let finalPath: string;
  let finalMdPath: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('fail-fast-foundation-workflow');
    draftPath = join(testDir, 'spec/foundation.json.draft');
    finalPath = join(testDir, 'spec/foundation.json');
    finalMdPath = join(testDir, 'spec/FOUNDATION.md');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Accepting a valid projectType on a draft and chaining to next field', () => {
    it('should succeed and include a system-reminder for the next unfilled field', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      // @step And the draft contains an unfilled projectType placeholder
      const draft = makeDraftWithProjectTypePlaceholder();
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run `fspec update-foundation projectType "web-app"`
      const result = await updateFoundation({
        section: 'projectType',
        content: 'web-app',
        cwd: testDir,
      });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);

      // @step And the draft file should contain `"projectType": "web-app"`
      const draftContent = await readFile(draftPath, 'utf-8');
      const parsed = JSON.parse(draftContent);
      expect(parsed.project.projectType).toBe('web-app');

      // @step And the response should include a system-reminder for the next unfilled field
      expect(result.systemReminder).toBeTruthy();
      expect(result.systemReminder).toContain('<system-reminder>');
    });
  });

  describe('Scenario: Fail-fast rejection of invalid problemImpact at write time', () => {
    it('should fail with a deterministic error listing valid values and leave draft unchanged', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      const draft = makeDraftWithProjectTypePlaceholder();
      const draftJson = JSON.stringify(draft, null, 2);
      await writeFile(draftPath, draftJson, 'utf-8');
      const draftStatBefore = await stat(draftPath);

      // @step When I run `fspec update-foundation problemImpact "critical"`
      const result = await updateFoundation({
        section: 'problemImpact',
        content: 'critical',
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.success).toBe(false);
      const errorText = result.error || '';

      // @step And the error output should contain `Invalid value for problemImpact: "critical"`
      expect(errorText).toContain(
        'Invalid value for problemImpact: "critical"'
      );

      // @step And the error output should list valid values: high, medium, low
      expect(errorText).toContain('high');
      expect(errorText).toContain('medium');
      expect(errorText).toContain('low');

      // @step And the error output should contain the text `Fix: fspec update-foundation problemImpact "<valid-value>"`
      expect(errorText).toContain(
        'Fix: fspec update-foundation problemImpact "<valid-value>"'
      );

      // @step And the draft file should be unchanged on disk
      const draftContentAfter = await readFile(draftPath, 'utf-8');
      expect(draftContentAfter).toBe(draftJson);
      const draftStatAfter = await stat(draftPath);
      expect(draftStatAfter.size).toBe(draftStatBefore.size);
    });
  });

  describe('Scenario: Accepting a freeform projectType that was previously rejected as not-in-enum', () => {
    it('should accept saas-platform (22 chars) and chain to next field', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      // @step And the draft contains an unfilled projectType placeholder
      const draft = makeDraftWithProjectTypePlaceholder();
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run `fspec update-foundation projectType "saas-platform"`
      const result = await updateFoundation({
        section: 'projectType',
        content: 'saas-platform',
        cwd: testDir,
      });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);

      // @step And the draft file should contain `"projectType": "saas-platform"`
      const parsed = JSON.parse(await readFile(draftPath, 'utf-8'));
      expect(parsed.project.projectType).toBe('saas-platform');

      // @step And the response should include a system-reminder for the next unfilled field
      expect(result.systemReminder).toBeTruthy();
      expect(result.systemReminder).toContain('<system-reminder>');
    });
  });

  describe('Scenario: Rejecting an empty projectType string', () => {
    it('should fail with an actionable length error and leave the draft unchanged', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      const draft = makeDraftWithProjectTypePlaceholder();
      const draftJson = JSON.stringify(draft, null, 2);
      await writeFile(draftPath, draftJson, 'utf-8');

      // @step When I run `fspec update-foundation projectType ""`
      const result = await updateFoundation({
        section: 'projectType',
        content: '',
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.success).toBe(false);
      const errorText = result.error || '';

      // @step And the error output should contain `Invalid projectType: "" (must be 1-30 characters, got 0)`
      expect(errorText).toContain(
        'Invalid projectType: "" (must be 1-30 characters, got 0)'
      );

      // @step And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
      expect(errorText).toContain(
        'Fix: fspec update-foundation projectType "<short-descriptor>"'
      );

      // @step And the draft file should be unchanged on disk
      const draftContentAfter = await readFile(draftPath, 'utf-8');
      expect(draftContentAfter).toBe(draftJson);
    });
  });

  describe('Scenario: Rejecting a projectType longer than 30 characters', () => {
    it('should fail with an actionable length error and leave the draft unchanged', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      const draft = makeDraftWithProjectTypePlaceholder();
      const draftJson = JSON.stringify(draft, null, 2);
      await writeFile(draftPath, draftJson, 'utf-8');
      const overlong =
        'a-very-long-project-type-descriptor-that-exceeds-the-limit';
      expect(overlong.length).toBe(58); // sanity check for scenario assertion

      // @step When I run `fspec update-foundation projectType "a-very-long-project-type-descriptor-that-exceeds-the-limit"`
      const result = await updateFoundation({
        section: 'projectType',
        content: overlong,
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.success).toBe(false);
      const errorText = result.error || '';

      // @step And the error output should contain `Invalid projectType: too long (must be 1-30 characters, got 58)`
      expect(errorText).toContain(
        'Invalid projectType: too long (must be 1-30 characters, got 58)'
      );

      // @step And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
      expect(errorText).toContain(
        'Fix: fspec update-foundation projectType "<short-descriptor>"'
      );

      // @step And the draft file should be unchanged on disk
      const draftContentAfter = await readFile(draftPath, 'utf-8');
      expect(draftContentAfter).toBe(draftJson);
    });
  });

  describe('Scenario: Accepting a freeform projectType like browser-extension', () => {
    it('should accept browser-extension (17 chars) on a draft', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      // @step And the draft contains an unfilled projectType placeholder
      const draft = makeDraftWithProjectTypePlaceholder();
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run `fspec update-foundation projectType "browser-extension"`
      const result = await updateFoundation({
        section: 'projectType',
        content: 'browser-extension',
        cwd: testDir,
      });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);

      // @step And the draft file should contain `"projectType": "browser-extension"`
      const parsed = JSON.parse(await readFile(draftPath, 'utf-8'));
      expect(parsed.project.projectType).toBe('browser-extension');
    });
  });

  describe('Scenario: Update-foundation on final foundation accepts a valid freeform projectType and regenerates markdown', () => {
    it('should rewrite foundation.json, regenerate FOUNDATION.md, and not emit discovery chaining', async () => {
      // @step Given no foundation draft exists
      expect(await fileExists(draftPath)).toBe(false);

      // @step And a final foundation.json exists with `"projectType": "cli-tool"`
      const foundation = createMinimalFoundation({
        project: {
          name: 'FinalProject',
          vision: 'Final foundation test',
          projectType: 'cli-tool',
        },
      });
      await writeFile(finalPath, JSON.stringify(foundation, null, 2), 'utf-8');
      await writeFile(finalMdPath, '# Old FOUNDATION.md\n', 'utf-8');
      const mdStatBefore = await stat(finalMdPath);

      // @step When I run `fspec update-foundation projectType "web-app"`
      const result = await updateFoundation({
        section: 'projectType',
        content: 'web-app',
        cwd: testDir,
      });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);

      // @step And the file spec/foundation.json should contain `"projectType": "web-app"`
      const parsed = JSON.parse(await readFile(finalPath, 'utf-8'));
      expect(parsed.project.projectType).toBe('web-app');

      // @step And the file spec/FOUNDATION.md should be regenerated
      const mdContentAfter = await readFile(finalMdPath, 'utf-8');
      expect(mdContentAfter).not.toBe('# Old FOUNDATION.md\n');
      const mdStatAfter = await stat(finalMdPath);
      expect(mdStatAfter.mtimeMs).toBeGreaterThanOrEqual(mdStatBefore.mtimeMs);

      // @step And the response should NOT include a discovery chaining system-reminder
      expect(result.systemReminder).toBeFalsy();
    });
  });

  describe('Scenario: Update-foundation on final foundation rejects an overlong projectType', () => {
    it('should fail and leave foundation.json and FOUNDATION.md unchanged', async () => {
      // @step Given no foundation draft exists
      expect(await fileExists(draftPath)).toBe(false);

      // @step And a final foundation.json exists with `"projectType": "cli-tool"`
      const foundation = createMinimalFoundation({
        project: {
          name: 'FinalProject',
          vision: 'Final foundation test',
          projectType: 'cli-tool',
        },
      });
      const foundationJson = JSON.stringify(foundation, null, 2);
      await writeFile(finalPath, foundationJson, 'utf-8');
      const originalMd = '# Original FOUNDATION.md\n\nOriginal content.\n';
      await writeFile(finalMdPath, originalMd, 'utf-8');

      const overlong =
        'a-very-long-project-type-descriptor-that-exceeds-the-limit';
      expect(overlong.length).toBe(58); // sanity check for scenario assertion

      // @step When I run `fspec update-foundation projectType "a-very-long-project-type-descriptor-that-exceeds-the-limit"`
      const result = await updateFoundation({
        section: 'projectType',
        content: overlong,
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.success).toBe(false);
      const errorText = result.error || '';

      // @step And the error output should contain `Invalid projectType: too long (must be 1-30 characters, got 58)`
      expect(errorText).toContain(
        'Invalid projectType: too long (must be 1-30 characters, got 58)'
      );

      // @step And the file spec/foundation.json should be unchanged on disk
      const foundationAfter = await readFile(finalPath, 'utf-8');
      expect(foundationAfter).toBe(foundationJson);

      // @step And the file spec/FOUNDATION.md should NOT be regenerated
      const mdAfter = await readFile(finalMdPath, 'utf-8');
      expect(mdAfter).toBe(originalMd);
    });
  });

  // ========================================================================
  // show-foundation --draft scenarios
  // ========================================================================

  describe('Scenario: Show foundation draft when draft exists', () => {
    it('should display the draft contents rendered like a final foundation', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      const draft = createMinimalFoundation({
        project: {
          name: 'DraftOnlyProject',
          vision: 'A project whose only state is the draft',
          projectType: 'cli-tool',
        },
      });
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step And no final foundation.json exists
      expect(await fileExists(finalPath)).toBe(false);

      // @step When I run `fspec show-foundation --draft`
      const result = await showFoundation({
        draft: true,
        cwd: testDir,
      });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);

      // @step And the output should display the draft contents rendered the same way as show-foundation renders a final foundation
      expect(result.output).toContain('DraftOnlyProject');
      expect(result.output).toContain(
        'A project whose only state is the draft'
      );
      expect(result.output).toContain('cli-tool');
    });
  });

  describe('Scenario: Show foundation draft when no draft exists', () => {
    it('should fail with clear error and suggest running discover-foundation', async () => {
      // @step Given no foundation draft exists at spec/foundation.json.draft
      expect(await fileExists(draftPath)).toBe(false);

      // @step When I run `fspec show-foundation --draft`
      const result = await showFoundation({
        draft: true,
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.success).toBe(false);

      // @step And the error output should contain `No draft found at spec/foundation.json.draft`
      expect(result.error).toContain(
        'No draft found at spec/foundation.json.draft'
      );

      // @step And the error output should suggest running `fspec discover-foundation` to create one
      expect(result.error).toContain('fspec discover-foundation');
    });
  });

  // ========================================================================
  // discover-foundation draft-exists & finalize error scenarios
  // ========================================================================

  describe('Scenario: Discover-foundation error when draft already exists', () => {
    it('should emit a system-reminder listing exactly three next-step options', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      const draft = {
        version: '2.0.0',
        project: {
          name: 'ExistingDraftProject',
          vision: '[QUESTION: What is the vision?]',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION: What problem?]',
            description: '[QUESTION: Describe the problem]',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: '[QUESTION: What can users DO?]',
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run `fspec discover-foundation`
      const result = await discoverFoundation({
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.valid).toBe(false);
      const reminder = result.systemReminder || '';

      // @step And the response should include a system-reminder listing exactly three next-step options
      expect(reminder).toContain('<system-reminder>');

      // @step And the system-reminder should contain `fspec discover-foundation --finalize`
      expect(reminder).toContain('fspec discover-foundation --finalize');

      // @step And the system-reminder should contain `fspec show-foundation --draft`
      expect(reminder).toContain('fspec show-foundation --draft');

      // @step And the system-reminder should contain `fspec discover-foundation --force`
      expect(reminder).toContain('fspec discover-foundation --force');

      // @step And the response should NOT include the raw draft content inline
      expect(reminder).not.toContain('ExistingDraftProject');
      expect(reminder).not.toContain('"version": "2.0.0"');
    });
  });

  describe('Scenario: Finalize fails with actionable length error when draft contains an overlong projectType', () => {
    it('should fail at finalization with a maxLength error instead of misleading missing-required error', async () => {
      // @step Given a foundation draft exists at spec/foundation.json.draft
      // @step And the draft contains all required fields filled with no placeholders
      // @step And the draft contains `"projectType": "a-ridiculously-long-string-way-beyond-thirty-characters"` written by manual edit
      const overlong =
        'a-ridiculously-long-string-way-beyond-thirty-characters';
      expect(overlong.length).toBe(55); // sanity check for scenario assertion
      const draft = {
        version: '2.0.0',
        project: {
          name: 'OverlongTypeProject',
          vision: 'A project whose type exceeds the 30-character limit',
          projectType: overlong,
        },
        problemSpace: {
          primaryProblem: {
            title: 'Overlong projectType',
            description: 'Test that finalization rejects overlong projectType',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution overview',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        personas: [
          {
            name: 'Test Persona',
            description: 'Test persona description',
            goals: ['Ship quality tests'],
          },
        ],
      };
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run `fspec discover-foundation --finalize`
      const result = await discoverFoundation({
        finalize: true,
        cwd: testDir,
      });

      // @step Then the command should exit with a non-zero code
      expect(result.valid).toBe(false);
      const errorText = result.validationErrors || '';

      // @step And the error output should contain `Invalid value at project.projectType`
      expect(errorText).toContain('Invalid value at project.projectType');

      // @step And the error output should contain `maxLength exceeded`
      expect(errorText).toContain('maxLength exceeded');

      // @step And the error output should contain `must be 1-30 characters, got 55`
      expect(errorText).toContain('must be 1-30 characters, got 55');

      // @step And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
      expect(errorText).toContain(
        'Fix: fspec update-foundation projectType "<short-descriptor>"'
      );

      // @step And the error output should NOT contain `Missing required: project.projectType`
      expect(errorText).not.toContain('Missing required: project.projectType');

      // @step And the draft file should NOT be deleted
      expect(await fileExists(draftPath)).toBe(true);

      // @step And no spec/foundation.json should be written
      expect(await fileExists(finalPath)).toBe(false);
    });
  });

  // ========================================================================
  // list-foundation-sections scenario
  // ========================================================================

  describe('Scenario: Discover valid foundation sections via list-foundation-sections', () => {
    it('should list every section with JSON path and constraint info', async () => {
      // @step When I run `fspec list-foundation-sections`
      const result = await listFoundationSections({ cwd: testDir });

      // @step Then the command should exit with code 0
      expect(result.success).toBe(true);
      const output = result.output || '';

      // @step And the output should list every valid section name
      const expectedSectionNames = [
        'projectName',
        'projectVision',
        'projectType',
        'problemTitle',
        'problemDefinition',
        'problemImpact',
        'solutionOverview',
      ];
      for (const section of expectedSectionNames) {
        expect(output).toContain(section);
      }

      // @step And the output should show each section's JSON path
      expect(output).toContain('project.name');
      expect(output).toContain('project.vision');
      expect(output).toContain('project.projectType');
      expect(output).toContain('problemSpace.primaryProblem.title');
      expect(output).toContain('problemSpace.primaryProblem.description');
      expect(output).toContain('problemSpace.primaryProblem.impact');
      expect(output).toContain('solutionSpace.overview');

      // @step And the output should describe `projectType` as `freeform string (1-30 characters)`
      expect(output).toContain('freeform string (1-30 characters)');

      // @step And the output should include non-exhaustive examples for projectType: cli-tool, web-app, saas-platform
      expect(output).toContain('cli-tool');
      expect(output).toContain('web-app');
      expect(output).toContain('saas-platform');

      // @step And the output should describe `problemImpact` as `enum: high, medium, low`
      expect(output).toContain('enum: high, medium, low');

      // @step And the output should describe other text fields as `freeform string`
      const freeformStringCount = (output.match(/freeform string/g) || [])
        .length;
      expect(freeformStringCount).toBeGreaterThanOrEqual(2);
    });
  });

  // ========================================================================
  // update-foundation help text scenario
  // ========================================================================

  describe('Scenario: Update-foundation help describes JSON field updates and lists all sections', () => {
    it('should describe command as updating a field in foundation.json and list all sections', () => {
      // @step When I run `fspec update-foundation --help`
      const helpText = renderHelpConfig(updateFoundationHelpConfig);

      // @step Then the command should exit with code 0
      expect(helpText.length).toBeGreaterThan(0);

      // @step And the output should describe the command as updating a field in foundation.json
      expect(helpText).toContain('foundation.json');
      expect(helpText).toMatch(/update.*field.*foundation\.json/i);

      // @step And the output should NOT contain the phrase `section content in FOUNDATION.md`
      expect(helpText).not.toContain('section content in FOUNDATION.md');

      // @step And the output should list all valid section names: projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
      const requiredSections = [
        'projectName',
        'projectVision',
        'projectType',
        'problemTitle',
        'problemDefinition',
        'problemImpact',
        'solutionOverview',
      ];
      for (const section of requiredSections) {
        expect(helpText).toContain(section);
      }

      // @step And the output should NOT use the abbreviation `etc.` when listing section names
      expect(helpText).not.toContain('etc.');

      // @step And the output should explicitly note that capabilities use the `add-capability` command
      expect(helpText).toContain('add-capability');

      // @step And the output should explicitly note that personas use the `add-persona` command
      expect(helpText).toContain('add-persona');
    });
  });
});
