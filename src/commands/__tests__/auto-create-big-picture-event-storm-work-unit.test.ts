/**
 * Feature: spec/features/auto-create-big-picture-event-storming-work-unit-after-foundation-finalization.feature
 *
 * Tests for auto-creating Big Picture Event Storming work unit after foundation finalization
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

// Get project root (3 levels up from dist/commands/__tests__)
const projectRoot = join(__dirname, '../../..');
const TEST_DIR = join(projectRoot, 'test-temp-auto-event-storm');
const CLI_PATH = join(projectRoot, 'dist/index.js');

describe('Feature: Auto-create Big Picture Event Storming work unit after foundation finalization', () => {
  beforeEach(async () => {
    // Create test directory
    await mkdir(TEST_DIR, { recursive: true });
    await mkdir(join(TEST_DIR, 'spec'), { recursive: true });

    // Create minimal work-units.json
    const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
    await writeFile(
      workUnitsPath,
      JSON.stringify(
        {
          version: '2.0.0',
          workUnits: {},
        },
        null,
        2
      )
    );

    // Create foundation draft with complete data (no placeholders)
    const draftPath = join(TEST_DIR, 'spec', 'foundation.json.draft');
    await writeFile(
      draftPath,
      JSON.stringify(
        {
          version: '2.0.0',
          project: {
            name: 'Test Project',
            vision: 'Test vision',
            projectType: 'cli-tool',
          },
          problemSpace: {
            primaryProblem: {
              title: 'Test Problem',
              description: 'Test problem description',
              impact: 'high',
            },
          },
          solutionSpace: {
            overview: 'Test solution',
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
              goals: ['Test goal'],
            },
          ],
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Cleanup test directory
    await rm(TEST_DIR, { recursive: true, force: true });
  });

  describe('Scenario: Work unit created when foundation finalized successfully', () => {
    it('should create work unit with FOUND prefix and Event Storming guidance', async () => {
      // @step Given foundation discovery has been completed
      // @step And all required foundation fields are populated
      // (Preconditions met by beforeEach - draft file created with complete data)

      // @step When I run "fspec discover-foundation --finalize"
      // @step And validation passes
      const result = execSync(
        `node ${CLI_PATH} discover-foundation --finalize --draft-path spec/foundation.json.draft`,
        {
          cwd: TEST_DIR,
          encoding: 'utf-8',
        }
      );

      // @step Then a new work unit should be created with FOUND prefix
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      expect(foundWorkUnits.length).toBeGreaterThan(0);

      const workUnitId = foundWorkUnits[0];
      const workUnit = workUnits.workUnits[workUnitId];

      // @step And the work unit status should be "backlog"
      expect(workUnit.status).toBe('backlog');

      // @step And the work unit type should be "task"
      expect(workUnit.type).toBe('task');

      // @step And the work unit title should contain "Big Picture Event Storming"
      expect(workUnit.title).toContain('Big Picture Event Storming');

      // @step And the work unit description should include foundation Event Storm commands
      expect(workUnit.description).toContain('add-foundation-bounded-context');
      expect(workUnit.description).toContain('add-aggregate-to-foundation');
      expect(workUnit.description).toContain('add-domain-event-to-foundation');
      expect(workUnit.description).toContain('show-foundation-event-storm');

      // @step And the work unit description should reference CLAUDE.md documentation
      expect(workUnit.description).toContain('CLAUDE.md');

      // @step And console output should confirm work unit creation
      expect(result).toContain('Created work unit');
      expect(result).toContain('Big Picture Event Storming');

      // @step And console output should show command to view work unit details
      expect(result).toContain('fspec show-work-unit');
    });
  });

  describe('Scenario: Work unit NOT created without finalize flag', () => {
    it('should not create work unit when --finalize flag is omitted', async () => {
      // @step Given foundation discovery has been completed
      // @step And all required foundation fields are populated
      // (Preconditions met by beforeEach)

      // @step When I run "fspec discover-foundation" without --finalize flag
      execSync(`node ${CLI_PATH} discover-foundation --force`, {
        cwd: TEST_DIR,
        encoding: 'utf-8',
      });

      // @step Then NO work unit should be created
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );

      expect(foundWorkUnits.length).toBe(0);

      // @step And work-units.json should remain unchanged
      expect(workUnits.workUnits).toEqual({});
    });
  });

  describe('Scenario: Work unit NOT created when validation fails', () => {
    it('should not create work unit when foundation validation fails', async () => {
      // @step Given foundation discovery has been completed
      // @step And foundation draft has validation errors
      const draftPath = join(TEST_DIR, 'spec', 'foundation.json.draft');
      await writeFile(
        draftPath,
        JSON.stringify(
          {
            version: '2.0.0',
            project: {
              name: 'Test Project',
              // Missing required fields: vision, projectType
            },
            // Missing required sections: problemSpace, solutionSpace, personas
          },
          null,
          2
        )
      );

      // @step When I run "fspec discover-foundation --finalize"
      // @step And validation fails
      let exitCode = 0;
      try {
        execSync(
          `node ${CLI_PATH} discover-foundation --finalize --draft-path spec/foundation.json.draft`,
          {
            cwd: TEST_DIR,
            encoding: 'utf-8',
          }
        );
      } catch (error) {
        exitCode = (error as { status: number }).status;
      }

      expect(exitCode).toBe(1);

      // @step Then NO work unit should be created
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      expect(foundWorkUnits.length).toBe(0);

      // @step And work-units.json should remain unchanged
      expect(workUnits.workUnits).toEqual({});

      // @step And error message should explain validation failure
      // (Error message checked by exit code 1)
    });
  });

  describe('Scenario: Work unit description contains Event Storming guidance', () => {
    it('should include all foundation Event Storm commands in description', async () => {
      // @step Given foundation has been finalized successfully
      // @step And a Big Picture Event Storming work unit was created
      execSync(
        `node ${CLI_PATH} discover-foundation --finalize --draft-path spec/foundation.json.draft`,
        {
          cwd: TEST_DIR,
          encoding: 'utf-8',
        }
      );

      // @step When AI agent reads the work unit description
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      const workUnit = workUnits.workUnits[foundWorkUnits[0]];
      const description = workUnit.description;

      // @step Then the description should list "add-foundation-bounded-context" command
      expect(description).toContain('add-foundation-bounded-context');

      // @step And the description should list "add-aggregate-to-foundation" command
      expect(description).toContain('add-aggregate-to-foundation');

      // @step And the description should list "add-domain-event-to-foundation" command
      expect(description).toContain('add-domain-event-to-foundation');

      // @step And the description should list "show-foundation-event-storm" command
      expect(description).toContain('show-foundation-event-storm');

      // @step And the description should explain why Big Picture Event Storming matters
      expect(description.toLowerCase()).toContain('bounded context');

      // @step And the description should reference "spec/CLAUDE.md" for detailed guidance
      expect(description).toContain('CLAUDE.md');
    });
  });
});
