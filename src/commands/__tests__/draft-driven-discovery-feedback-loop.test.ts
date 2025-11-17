/**
 * Feature: spec/features/implement-draft-driven-discovery-workflow-with-ai-chaining.feature
 * Work Unit: DISC-001
 *
 * This test file validates the draft-driven discovery workflow with AI chaining feedback loop.
 * Tests map directly to scenarios in the Gherkin feature file.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm, stat } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { mkdtemp } from 'fs/promises';
import { discoverFoundation } from '../discover-foundation';
import type { GenericFoundation } from '../../types/generic-foundation';

describe('Feature: Implement draft-driven discovery workflow with AI chaining', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Initial draft creation with ULTRATHINK guidance', () => {
    it('should create foundation.json.draft with REQUIRED field structure and [QUESTION:] placeholders', async () => {
      // Given human runs "fspec discover-foundation"
      // When command creates foundation.json.draft
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
        cwd: tmpDir,
        force: true,
      });

      // Then draft should contain REQUIRED field structure
      expect(result.draftCreated).toBe(true);
      expect(result.draftContent).toBeDefined();

      const draft = JSON.parse(result.draftContent);
      expect(draft.project).toBeDefined();
      expect(draft.problemSpace).toBeDefined();
      expect(draft.solutionSpace).toBeDefined();
      expect(draft.personas).toBeDefined();

      // And project fields should have [QUESTION:] placeholders for name, vision
      expect(draft.project.name).toContain('[QUESTION:');
      expect(draft.project.vision).toContain('[QUESTION:');

      // And project.projectType should have [DETECTED:] placeholder
      expect(draft.project.projectType).toContain('[DETECTED:');

      // And problemSpace should have [QUESTION:] placeholders
      expect(draft.problemSpace.primaryProblem.title).toContain('[QUESTION:');

      // And solutionSpace should have [QUESTION:] placeholders
      expect(draft.solutionSpace.overview).toContain('[QUESTION:');

      // And personas array should have [QUESTION:] placeholders
      expect(draft.personas[0].name).toContain('[QUESTION:');
    });

    it('should emit system-reminder with ULTRATHINK guidance and field 1/N progress', async () => {
      process.env.FSPEC_AGENT = 'claude';
      // When command creates draft
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
        cwd: tmpDir,
        force: true,
      });
      delete process.env.FSPEC_AGENT;

      // Then command should emit system-reminder with ULTRATHINK guidance
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // And system-reminder should instruct AI to analyze entire codebase
      expect(result.systemReminder).toContain('ULTRATHINK');
      expect(result.systemReminder).toContain('Analyze');

      // And system-reminder should show "Field 1/N: project.name"
      expect(result.systemReminder).toMatch(/Field 1\/\d+/);
      expect(result.systemReminder).toContain('project.name');
    });
  });

  describe('Scenario: Fill project.name field via feedback loop', () => {
    it('should emit system-reminder for project.name with exact command to run', async () => {
      // Given foundation.json.draft exists with [QUESTION: What is the project name?]
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: '[QUESTION: What is the project name?]',
          vision: '[QUESTION: What is the vision?]',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command scans draft for next unfilled field
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should emit system-reminder for project.name
      expect(result.systemReminder).toContain('project.name');

      // And system-reminder should say "Field 1/N: project.name"
      expect(result.systemReminder).toMatch(/Field 1\/\d+: project\.name/);

      // And system-reminder should instruct to analyze project configuration
      expect(result.systemReminder).toContain('Analyze project configuration');

      // And system-reminder should provide exact command to run
      expect(result.systemReminder).toContain('fspec update-foundation');
      expect(result.systemReminder).toContain('projectName');
    });

    it('should detect draft update and automatically re-scan for next unfilled field', async () => {
      // Given draft has project.name filled
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec', // FILLED
          vision: '[QUESTION: What is the vision?]', // NEXT UNFILLED
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When AI runs "fspec update-foundation --field project.name --value fspec"
      // Then command should detect draft update
      // And command should automatically re-scan draft
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // And command should find NEXT unfilled field (project.vision)
      expect(result.nextField).toBe('project.vision');
      expect(result.systemReminder).toContain('project.vision');
    });
  });

  describe('Scenario: Fill project.vision with ULTRATHINK guidance', () => {
    it('should emit system-reminder emphasizing ULTRATHINK and WHY not HOW', async () => {
      process.env.FSPEC_AGENT = 'claude';
      // Given draft has project.name filled
      // And draft has [QUESTION: What is the one-sentence vision?] for project.vision
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: '[QUESTION: What is the one-sentence vision?]',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans draft
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });
      delete process.env.FSPEC_AGENT;

      // Then command should emit system-reminder for project.vision
      expect(result.systemReminder).toContain('project.vision');

      // And system-reminder should say "ULTRATHINK: Read ALL code, understand deeply"
      expect(result.systemReminder).toContain('ULTRATHINK');
      expect(result.systemReminder).toContain('Read ALL code');

      // And system-reminder should emphasize WHY not HOW
      expect(result.systemReminder).toContain('WHY');
      expect(result.systemReminder).toContain('not HOW');

      // And system-reminder should instruct AI to formulate elevator pitch
      expect(result.systemReminder).toContain('elevator pitch');
    });

    it('should chain to next unfilled field after vision is updated', async () => {
      // Given draft has project.vision filled
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs with ACDD',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should chain to next unfilled field (project.projectType)
      expect(result.nextField).toBe('project.projectType');
    });
  });

  describe('Scenario: Verify DETECTED project type with human', () => {
    it('should emit system-reminder to verify detected value with all projectType options', async () => {
      // Given draft has [DETECTED: cli-tool] for project.projectType
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command scans draft
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should emit system-reminder to verify detected value
      expect(result.systemReminder).toContain('[DETECTED: cli-tool]');
      expect(result.systemReminder).toContain('Verify');

      // And system-reminder should list all projectType options
      expect(result.systemReminder).toContain('cli-tool');
      expect(result.systemReminder).toContain('web-app');
      expect(result.systemReminder).toContain('library');

      // And system-reminder should instruct AI to verify with human
      expect(result.systemReminder).toContain('Verify with human');
    });

    it('should accept verified value and chain to problemSpace fields', async () => {
      // Given AI confirms projectType with human
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool', // VERIFIED (no [DETECTED:] tag)
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should accept verified value
      // And command should chain to problemSpace fields
      expect(result.nextField).toContain('problemSpace');
    });
  });

  describe('Scenario: Fill problemSpace from USER perspective', () => {
    it('should emit system-reminder emphasizing USER perspective and WHO/WHAT/WHY', async () => {
      // Given draft has [QUESTION: What problem does this solve?]
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION: What problem does this solve?]',
            description: '[QUESTION: What problem does this solve?]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command scans problemSpace.primaryProblem
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then system-reminder should emphasize USER perspective
      expect(result.systemReminder).toContain('USER perspective');

      // And system-reminder should ask "WHO uses this? (persona)"
      expect(result.systemReminder).toContain('WHO uses this');

      // And system-reminder should ask "WHAT problem do THEY face? (WHY)"
      expect(result.systemReminder).toContain('WHAT problem');

      // And system-reminder should require title, description, impact
      expect(result.systemReminder).toContain('title');
      expect(result.systemReminder).toContain('description');
      expect(result.systemReminder).toContain('impact');
    });

    it('should chain to solutionSpace after problemSpace is filled', async () => {
      // Given AI fills primaryProblem fields
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description:
              'AI agents need structured workflow for specifications',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: '[QUESTION: What can users DO?]',
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should chain to solutionSpace
      expect(result.nextField).toContain('solutionSpace');
    });
  });

  describe('Scenario: Fill solutionSpace.capabilities with WHAT not HOW focus', () => {
    it('should emit system-reminder emphasizing WHAT not HOW with examples', async () => {
      // Given draft has [QUESTION: What can users DO?] for capabilities
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: '[QUESTION: What can users DO?]',
        },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command scans solutionSpace.capabilities
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then system-reminder should emphasize WHAT not HOW
      expect(result.systemReminder).toContain('WHAT not HOW');

      // And system-reminder should give example: "Spec Validation" not "Uses Cucumber parser"
      expect(result.systemReminder).toContain('Spec Validation');
      expect(result.systemReminder).toContain('NOT');
      expect(result.systemReminder).toContain('Cucumber parser');

      // And system-reminder should instruct to list 3-7 high-level abilities
      expect(result.systemReminder).toContain('3-7');
      expect(result.systemReminder).toContain('high-level abilities');
    });

    it('should chain to personas after capabilities are filled', async () => {
      // Given AI fills capabilities
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: '[QUESTION: Who uses this?]',
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should chain to personas
      expect(result.nextField).toContain('personas');
    });
  });

  describe('Scenario: Fill personas from user interaction analysis', () => {
    it('should emit system-reminder instructing to identify ALL user types from interactions', async () => {
      // Given draft has [QUESTION: Who uses this?] for personas
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: '[QUESTION: Who uses this?]',
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command scans personas array
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then system-reminder should instruct to identify ALL user types
      expect(result.systemReminder).toContain('Identify ALL user types');

      // And system-reminder should guide based on project type
      expect(result.systemReminder).toContain('CLI');

      // And system-reminder should ask about goals and pain points
      expect(result.systemReminder).toContain('goals');
      expect(result.systemReminder).toContain('pain points');
    });

    it('should recognize personas as last required field', async () => {
      // Given AI fills persona
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: [
          {
            name: 'Developer using CLI',
            description: 'Uses fspec in terminal',
            goals: ['Manage specifications'],
          },
        ],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // When command re-scans
      const result = await discoverFoundation({
        draftPath,
        scanOnly: true,
      });

      // Then command should recognize this as last required field
      expect(result.allFieldsComplete).toBe(true);
    });
  });

  describe('Scenario: Complete discovery with auto-generation', () => {
    it('should validate draft, create foundation.json, delete draft, and auto-generate FOUNDATION.md', async () => {
      // Given AI has filled all required fields
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const completeDraft: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs with ACDD',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description:
              'AI agents need structured workflow for specifications',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based specification management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: [
          {
            name: 'Developer using CLI',
            description: 'Uses fspec in terminal',
            goals: ['Manage specifications'],
          },
        ],
      };
      await writeFile(
        draftPath,
        JSON.stringify(completeDraft, null, 2),
        'utf-8'
      );

      // When command re-scans draft
      const result = await discoverFoundation({
        draftPath,
        cwd: tmpDir,
        finalize: true,
        outputPath: join(tmpDir, 'spec', 'foundation.json'),
        autoGenerateMd: true,
      });

      // Then command should find NO [QUESTION:] placeholders in required fields
      expect(result.allFieldsComplete).toBe(true);

      // And command should validate draft against JSON schema
      expect(result.validated).toBe(true);

      // And validation should PASS
      expect(result.valid).toBe(true);

      // Then command should create spec/foundation.json from draft
      expect(result.finalCreated).toBe(true);
      const foundationExists = await stat(
        join(tmpDir, 'spec', 'foundation.json')
      )
        .then(() => true)
        .catch(() => false);
      expect(foundationExists).toBe(true);

      // And command should delete spec/foundation.json.draft
      expect(result.draftDeleted).toBe(true);
      const draftExists = await stat(draftPath)
        .then(() => true)
        .catch(() => false);
      expect(draftExists).toBe(false);

      // And command should AUTOMATICALLY run "fspec generate-foundation-md"
      expect(result.mdGenerated).toBe(true);

      // And command should emit completion message
      expect(result.completionMessage).toContain('Discovery complete');
      expect(result.completionMessage).toContain('foundation.json');
      expect(result.completionMessage).toContain('FOUNDATION.md');
    });
  });

  describe('Scenario: Detect and reject manual editing', () => {
    it('should detect draft mtime changed outside fspec and emit ERROR system-reminder', async () => {
      // Given foundation.json.draft exists with unfilled fields
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const draftContent = {
        version: '2.0.0',
        project: {
          name: '[QUESTION: What is the project name?]',
          vision: '[QUESTION: What is the vision?]',
          projectType: '[DETECTED: cli-tool]',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION:]',
            description: '[QUESTION:]',
            impact: 'high',
          },
        },
        solutionSpace: { overview: '[QUESTION:]', capabilities: [] },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(draftContent, null, 2),
        'utf-8'
      );

      // Store last known state
      const lastKnownContent = JSON.stringify(draftContent);

      // When AI attempts to edit draft using Write tool directly
      const manuallyEditedContent = {
        ...draftContent,
        project: { ...draftContent.project, name: 'manually-edited-name' },
      };
      await writeFile(
        draftPath,
        JSON.stringify(manuallyEditedContent, null, 2),
        'utf-8'
      );

      // When command detects change
      const result = await discoverFoundation({
        draftPath,
        lastKnownState: lastKnownContent,
        detectManualEdit: true,
      });

      // Then command should detect draft mtime changed outside fspec
      expect(result.manualEditDetected).toBe(true);

      // And command should emit ERROR in system-reminder
      expect(result.errorReminder).toContain('<system-reminder>');
      expect(result.errorReminder).toContain('ERROR');

      // And error should say "CRITICAL: You manually edited foundation.json.draft"
      expect(result.errorReminder).toContain('CRITICAL');
      expect(result.errorReminder).toContain('manually edited');

      // And error should say "You MUST use: fspec update-foundation, fspec add-capability, fspec add-persona"
      expect(result.errorReminder).toContain('fspec update-foundation');
      expect(result.errorReminder).toContain('fspec add-capability');
      expect(result.errorReminder).toContain('fspec add-persona');

      // And command should revert changes
      expect(result.reverted).toBe(true);

      // And draft should be restored to last valid state
      const restoredContent = await readFile(draftPath, 'utf-8');
      expect(restoredContent).toBe(lastKnownContent);
    });
  });

  describe('Scenario: Handle schema validation failure', () => {
    it('should emit schema validation error with missing field details and keep draft', async () => {
      // Given AI has filled all required fields
      // When command validates draft against schema
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const invalidDraft = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            // Missing description field (REQUIRED)
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: [
          {
            name: 'Developer using CLI',
            description: 'Uses fspec in terminal',
            goals: ['Manage specs'],
          },
        ],
      };
      await writeFile(
        draftPath,
        JSON.stringify(invalidDraft, null, 2),
        'utf-8'
      );

      // When command validates draft
      const result = await discoverFoundation({
        draftPath,
        cwd: tmpDir,
        finalize: true,
        outputPath: join(tmpDir, 'spec', 'foundation.json'),
      });

      // Then validation should FAIL with missing field: problemSpace.primaryProblem.description
      expect(result.validated).toBe(true);
      expect(result.valid).toBe(false);

      // And error should say "Missing required: problemSpace.primaryProblem.description"
      expect(result.validationErrors).toContain(
        'problemSpace.primaryProblem.description'
      );

      // And error should provide fix command with appropriate commands
      expect(result.validationErrors).toContain('fspec update-foundation');
      expect(result.validationErrors).toContain('fspec add-capability');
      expect(result.validationErrors).toContain('fspec add-persona');

      // And draft should NOT be deleted
      const draftExists = await stat(draftPath)
        .then(() => true)
        .catch(() => false);
      expect(draftExists).toBe(true);

      // And foundation.json should NOT be created
      expect(result.finalCreated).toBeUndefined();
      const foundationExists = await stat(
        join(tmpDir, 'spec', 'foundation.json')
      )
        .then(() => true)
        .catch(() => false);
      expect(foundationExists).toBe(false);
    });

    it('should validate again and succeed after missing field is fixed', async () => {
      // Given AI fixes missing field
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const fixedDraft: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description:
              'AI agents need structured workflow for specifications', // FIXED
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based spec management',
          capabilities: [
            {
              name: 'Gherkin Validation',
              description: 'Validate feature files',
            },
          ],
        },
        personas: [
          {
            name: 'Developer using CLI',
            description: 'Uses fspec in terminal',
            goals: ['Manage specs'],
          },
        ],
      };
      await writeFile(draftPath, JSON.stringify(fixedDraft, null, 2), 'utf-8');

      // When AI re-runs discover-foundation
      const result = await discoverFoundation({
        draftPath,
        cwd: tmpDir,
        finalize: true,
        outputPath: join(tmpDir, 'spec', 'foundation.json'),
      });

      // Then command should validate again and succeed
      expect(result.validated).toBe(true);
      expect(result.valid).toBe(true);
      expect(result.finalCreated).toBe(true);
    });
  });
});
